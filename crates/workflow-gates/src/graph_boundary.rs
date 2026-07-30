//! Structural `gandr-graph` boundary analysis.
//!
//! # Contract
//! - requires: fixture metadata, when supplied, has Cargo-compatible `packages`
//!   and `workspace_members` fields whose member package manifests are absolute
//!   or relative to the workspace root.
//! - ensures: returns findings for graph-stack direct dependencies outside
//!   `gandr-graph`, graph-stack source references outside `gandr-graph`, and
//!   externally visible `gandr-graph` declarations exposing graph-stack roots.
//! - provides: structural `syn`-backed graph-boundary diagnostics for workspace
//!   packages.
//! - fails: returns typed gate errors for unreadable metadata/source files,
//!   failed live Cargo metadata, invalid metadata JSON or shape, ambiguous or
//!   missing public module sources, and Rust parse failures.
//! - panics: none.
//! - intension: returned finding order is the declared projection for stable
//!   finding sort, symlink-bounded source walking, declared public-module graph
//!   traversal, scope alias resolution, and structural `syn` traversal.
//!
//! # Adequacy
//! - hypothesis: L3 only — fixture workspaces separate clean metadata, direct
//!   dependency leaks, outside-source leaks, inline and path-overridden public
//!   module reachability, alias resolution, forbidden wildcard imports, impl
//!   self-type nameability, additional public item forms, symlink bounding,
//!   relative manifests, owner-private implementation uses, Rust parse
//!   failures, and sorting by exact finding vectors or typed error
//!   observations.
//! - witness: `graph_boundary::tests::clean_metadata_and_sources_have_no_findings`
//! - witness: `graph_boundary::tests::direct_petgraph_and_fixedbitset_dependencies_are_rejected`
//! - witness: `graph_boundary::tests::private_outside_crate_graph_stack_use_is_rejected`
//! - witness: `graph_boundary::tests::public_module_graph_controls_api_analysis`
//! - witness: `graph_boundary::tests::ambiguous_public_module_sources_are_operational_errors`
//! - witness: `graph_boundary::tests::inline_module_directories_resolve_nested_out_of_line_modules`
//! - witness: `graph_boundary::tests::path_overridden_modules_resolve_from_effective_module_directory`
//! - witness: `graph_boundary::tests::restricted_visibility_and_private_adapters_are_not_public_api`
//! - witness: `graph_boundary::tests::alias_and_impl_surfaces_are_rejected`
//! - witness: `graph_boundary::tests::private_forbidden_glob_cannot_hide_public_signature_root`
//! - witness: `graph_boundary::tests::forbidden_glob_inside_private_module_does_not_leak`
//! - witness: `graph_boundary::tests::private_self_trait_impl_is_not_public_api`
//! - witness: `graph_boundary::tests::public_self_trait_impl_is_public_api`
//! - witness: `graph_boundary::tests::additional_public_type_bearing_forms_are_rejected`
//! - witness: `graph_boundary::tests::symlinked_source_entries_are_bounded`
//! - witness: `graph_boundary::tests::relative_manifest_paths_resolve_from_workspace_root`
//! - witness: `graph_boundary::tests::rust_parse_errors_are_operational_errors`
//! - witness: `graph_boundary::tests::findings_are_deterministically_ordered`

/// Allocation collections used in std-backed gate code.
extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Map;
use serde_json::Value;
use syn::Fields;
use syn::ImplItem;
use syn::Item;
use syn::TraitItem;
use syn::UseTree;
use syn::Visibility;
use syn::visit::Visit;

use crate::Finding;
use crate::GateError;
use crate::GateResult;

crate::semantic_str!(pub struct PackageText);
crate::semantic_str!(pub struct SourceText);
crate::semantic_str!(pub struct FieldText);
crate::semantic_str!(pub struct ContextText);
crate::semantic_optional_str!(pub struct OptionalInheritedRootText);
crate::semantic_str!(pub struct KindText);
crate::semantic_str!(pub struct PathText);
crate::semantic_str!(pub struct DetailText);
crate::semantic_str!(pub struct MetadataJsonText);
crate::semantic_copy!(pub struct ParentPublicFlag(bool));
crate::semantic_str!(pub struct NameText);
crate::semantic_copy!(pub struct CollectUseAliasesFlag(bool));
crate::semantic_copy!(pub struct SelfTypeIsPublicFlag(bool));
crate::semantic_copy!(pub struct BuiltinTypeNameFlag(bool));
crate::semantic_copy!(pub struct VisibleApiFlag(bool));
crate::semantic_copy!(pub struct GraphStackNameFlag(bool));
crate::semantic_copy!(pub struct PathIsRegularSourceFlag(bool));

/// Workspace package allowed to own graph-stack implementation dependencies.
const GRAPH_PACKAGE: &str = "gandr-graph";
/// Stable detail for direct dependency violations.
const DIRECT_DEPENDENCY_DETAIL: &str =
    "petgraph/fixedbitset may be direct dependencies only of gandr-graph";
/// Stable detail for non-graph source references to graph-stack crates.
const OUTSIDE_SOURCE_DETAIL: &str =
    "source outside gandr-graph must not mention petgraph/fixedbitset graph-stack APIs";
/// Stable detail for public `gandr-graph` API exposure violations.
const PUBLIC_API_DETAIL: &str =
    "public gandr-graph declarations must not expose petgraph/fixedbitset graph-stack APIs";

/// Runs graph-boundary analysis for a workspace root.
///
/// # Contract
/// - requires: `workspace_root` names the workspace to analyze; when
///   `metadata_fixture` is supplied, it names readable Cargo metadata JSON.
/// - ensures: returns the same findings as [`analyze_workspace`] for the
///   selected metadata.
/// - provides: graph-boundary findings for one workspace root.
/// - fails: returns typed gate errors for unreadable metadata fixtures, failed
///   live Cargo metadata, invalid metadata JSON or shape, unreadable source
///   files, and Rust parse failures.
/// - panics: none.
/// - intension: returned finding order is the declared projection for the
///   downstream stable analysis order.
///
/// # Errors
/// Returns I/O errors for unreadable metadata fixtures, operational errors for
/// failed live Cargo metadata, JSON errors for invalid metadata, operational
/// errors for invalid metadata shape, I/O errors for unreadable source files,
/// and Rust parse errors for invalid source files.
///
/// # Adequacy
/// - hypothesis: L3 only — fixture-backed workspaces separate clean runs,
///   direct dependency findings, source findings, public API findings, Rust
///   parse failure, and sorted finding order through exact returned findings
///   and typed error observations.
/// - witness: `graph_boundary::tests::clean_metadata_and_sources_have_no_findings`
/// - witness: `graph_boundary::tests::direct_petgraph_and_fixedbitset_dependencies_are_rejected`
/// - witness: `graph_boundary::tests::private_outside_crate_graph_stack_use_is_rejected`
/// - witness: `graph_boundary::tests::multiline_nested_grouped_and_renamed_public_uses_are_rejected`
/// - witness: `graph_boundary::tests::public_signature_type_and_trait_leaks_are_rejected`
/// - witness: `graph_boundary::tests::private_implementation_inside_gandr_graph_does_not_leak`
/// - witness: `graph_boundary::tests::rust_parse_errors_are_operational_errors`
/// - witness: `graph_boundary::tests::findings_are_deterministically_ordered`
#[inline]
pub fn run(
    workspace_root: &Path,
    metadata_fixture: Option<&Path>,
) -> GateResult
{
    let metadata_json = match metadata_fixture {
        | Some(path) => read_text(path)?,
        | None => cargo_metadata(workspace_root)?,
    };

    analyze_workspace(workspace_root, &metadata_json)
}

/// Analyze one Rust source string for graph-boundary leaks.
///
/// # Contract
/// - requires: `path` names the supplied source for diagnostics; `package`
///   selects outside-source checks unless it is exactly `gandr-graph`.
/// - ensures: returns source findings outside `gandr-graph` and public API
///   findings inside `gandr-graph`.
/// - provides: graph-boundary findings for one already captured Rust source
///   file.
/// - fails: returns a Rust parse error when `source` is not a complete Rust
///   file.
/// - panics: none.
/// - intension: returned finding order is the declared projection for
///   structural `syn` traversal and finding sort by kind, package, path,
///   declaration, then detail.
///
/// # Errors
/// Returns a Rust parse error when `source` is not a complete Rust file.
///
/// # Adequacy
/// - hypothesis: L3 only — integration fixtures distinguish outside-source
///   checking from `gandr-graph` public API checking, alias/wildcard
///   resolution, impl-self nameability, and parse failure through exact finding
///   vectors and typed error observations.
/// - witness: `graph_boundary::tests::private_outside_crate_graph_stack_use_is_rejected`
/// - witness: `graph_boundary::tests::multiline_nested_grouped_and_renamed_public_uses_are_rejected`
/// - witness: `graph_boundary::tests::public_signature_type_and_trait_leaks_are_rejected`
/// - witness: `graph_boundary::tests::private_implementation_inside_gandr_graph_does_not_leak`
/// - witness: `graph_boundary::tests::private_forbidden_glob_cannot_hide_public_signature_root`
/// - witness: `graph_boundary::tests::forbidden_glob_inside_private_module_does_not_leak`
/// - witness: `graph_boundary::tests::private_self_trait_impl_is_not_public_api`
/// - witness: `graph_boundary::tests::public_self_trait_impl_is_public_api`
/// - witness: `graph_boundary::tests::rust_parse_errors_are_operational_errors`
#[inline]
pub fn analyze_source<'semantic, P, S>(
    path: &Path,
    package: P,
    source: S,
) -> GateResult
where
    P: Into<PackageText<'semantic>>,
    S: Into<SourceText<'semantic>>,
{
    let source = source.into().0;
    let package = package.into().0;
    let syntax = syn::parse_file(source).map_err(|error| GateError::RustParse {
        path: path.to_path_buf(),
        source: error,
    })?;
    let mut findings = Vec::new();
    if package == GRAPH_PACKAGE {
        graph_public_api_findings(Path::new(""), package, path, &syntax, &mut findings)?;
    }
    else {
        outside_source_findings(Path::new(""), package, path, &syntax, &mut findings);
    }
    findings.sort_by(finding_cmp);
    Ok(findings)
}

/// Parse the subset of Cargo metadata needed for graph-boundary analysis.
fn parse_metadata<'semantic, J>(metadata_json: J) -> Result<Metadata, GateError>
where
    J: Into<MetadataJsonText<'semantic>>,
{
    let metadata_json = metadata_json.into().0;
    let value: Value = serde_json::from_str(metadata_json).map_err(|error| GateError::Json {
        source_name: String::from("cargo metadata"),
        source: error,
    })?;
    let metadata_object = value_object(&value, "metadata root")?;

    let packages = get_array_field(metadata_object, "packages", "metadata root")?
        .iter()
        .map(parse_metadata_package)
        .collect::<Result<Vec<_>, _>>()?;
    let workspace_members = get_array_field(metadata_object, "workspace_members", "metadata root")?
        .iter()
        .map(|member| value_string(member, "workspace member"))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Metadata {
        packages,
        workspace_members,
    })
}

/// Parse one package object from Cargo metadata.
fn parse_metadata_package(package_value: &Value) -> Result<MetadataPackage, GateError>
{
    let object = value_object(package_value, "package")?;
    let id = get_string_field(object, "id", "package")?;
    let name = get_string_field(object, "name", "package")?;
    let manifest_path = PathBuf::from(get_string_field(object, "manifest_path", "package")?);
    let lib_src_path = match object.get("targets") {
        | Some(targets) => parse_library_target_path(value_array(targets, "package targets")?)?,
        | None => None,
    };
    let dependencies = match object.get("dependencies") {
        | Some(dependency_values) => value_array(dependency_values, "package dependencies")?
            .iter()
            .map(parse_metadata_dependency)
            .collect::<Result<Vec<_>, _>>()?,
        | None => Vec::new(),
    };

    Ok(MetadataPackage {
        id,
        name,
        manifest_path,
        lib_src_path,
        dependencies,
    })
}

/// Parse one dependency object from Cargo metadata.
fn parse_metadata_dependency(dependency_value: &Value) -> Result<MetadataDependency, GateError>
{
    let object = value_object(dependency_value, "dependency")?;
    Ok(MetadataDependency {
        name: get_string_field(object, "name", "dependency")?,
    })
}

/// Read a required array field from a JSON object.
fn get_array_field<'semantic, 'value, F, C>(
    object: &'value Map<String, Value>,
    field: F,
    context: C,
) -> Result<&'value [Value], GateError>
where
    F: Into<FieldText<'semantic>>,
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    let field = field.into().0;
    let value = object
        .get(field)
        .ok_or_else(|| operational(format!("missing {field} in {context}")))?;
    value_array(value, field)
}

/// Interpret a JSON value as an object.
fn value_object<'semantic, 'value, C>(
    value: &'value Value,
    context: C,
) -> Result<&'value Map<String, Value>, GateError>
where
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    value
        .as_object()
        .ok_or_else(|| operational(format!("expected object for {context}")))
}

/// Read a required string field from a JSON object.
fn get_string_field<'semantic, F, C>(
    object: &Map<String, Value>,
    field: F,
    context: C,
) -> Result<String, GateError>
where
    F: Into<FieldText<'semantic>>,
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    let field = field.into().0;
    let value = object
        .get(field)
        .ok_or_else(|| operational(format!("missing {field} in {context}")))?;
    value_string(value, field)
}

/// Parse the library target source path from Cargo metadata targets.
fn parse_library_target_path(targets: &[Value]) -> Result<Option<PathBuf>, GateError>
{
    for target in targets {
        let object = value_object(target, "package target")?;
        let kinds = get_array_field(object, "kind", "package target")?;
        let is_library = kinds
            .iter()
            .map(|kind| value_string(kind, "target kind"))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|kind| kind == "lib");
        if is_library {
            return Ok(Some(PathBuf::from(get_string_field(
                object,
                "src_path",
                "package target",
            )?)));
        }
    }
    Ok(None)
}

/// Interpret a JSON value as an array.
fn value_array<'semantic, 'value, C>(
    value: &'value Value,
    context: C,
) -> Result<&'value [Value], GateError>
where
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| operational(format!("expected array for {context}")))
}

/// Interpret a JSON value as a string.
fn value_string<'semantic, C>(
    value: &Value,
    context: C,
) -> Result<String, GateError>
where
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| operational(format!("expected string for {context}")))
}

/// Select workspace packages and sort them deterministically.
fn workspace_packages(metadata: Metadata) -> Vec<MetadataPackage>
{
    let members: BTreeSet<String> = metadata.workspace_members.into_iter().collect();
    let mut packages = metadata
        .packages
        .into_iter()
        .filter(|package| members.contains(&package.id))
        .collect::<Vec<_>>();

    packages.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.manifest_path.cmp(&right.manifest_path))
    });

    packages
}

/// Append direct petgraph/fixedbitset dependency findings for one package.
fn direct_dependency_findings(
    workspace_root: &Path,
    package: &MetadataPackage,
    findings: &mut Vec<Finding>,
)
{
    if package.name == GRAPH_PACKAGE {
        return;
    }

    let manifest_path = resolved_manifest_path(workspace_root, &package.manifest_path);
    for dependency in &package.dependencies {
        if is_graph_stack_name(&dependency.name).into().0 {
            findings.push(finding(
                "direct-dependency",
                &package.name,
                relative_path(workspace_root, &manifest_path),
                dependency.name.clone(),
                DIRECT_DEPENDENCY_DETAIL,
            ));
        }
    }
}

/// Parse and analyze the Rust source files owned by one workspace package.
fn source_findings(
    workspace_root: &Path,
    package: &MetadataPackage,
    findings: &mut Vec<Finding>,
) -> Result<(), GateError>
{
    let manifest_path = resolved_manifest_path(workspace_root, &package.manifest_path);
    let package_root = manifest_path.parent().ok_or_else(|| {
        operational(format!(
            "manifest path has no parent for package {}: {}",
            package.name,
            manifest_path.display()
        ))
    })?;

    if package.name == GRAPH_PACKAGE {
        let library_root = package.lib_src_path.as_ref().map_or_else(
            || package_root.join("src/lib.rs"),
            |path| resolved_manifest_path(workspace_root, path),
        );
        let syntax = parse_source_file(&library_root)?;
        graph_public_api_findings(
            workspace_root,
            &package.name,
            &library_root,
            &syntax,
            findings,
        )?;
    }
    else {
        for source_path in rust_source_files(package_root)? {
            let syntax = parse_source_file(&source_path)?;
            outside_source_findings(
                workspace_root,
                &package.name,
                &source_path,
                &syntax,
                findings,
            );
        }
    }

    Ok(())
}

/// Append public API boundary findings for one parsed `gandr-graph` library
/// root and its externally visible declared modules.
fn graph_public_api_findings<'semantic, P>(
    workspace_root: &Path,
    package: P,
    source_path: &Path,
    syntax: &syn::File,
    findings: &mut Vec<Finding>,
) -> Result<(), GateError>
where
    P: Into<PackageText<'semantic>>,
{
    let package = package.into().0;
    let source_dir = source_path.parent().ok_or_else(|| {
        operational(format!(
            "library root has no parent for package {package}: {}",
            source_path.display()
        ))
    })?;
    inspect_module_items(
        &ModuleInspection {
            workspace_root,
            package,
            module_dir: source_dir,
            path: relative_path(workspace_root, source_path),
            items: &syntax.items,
            parent_public: true,
            inherited_aliases: &AliasContext::default(),
        },
        findings,
    )
}

/// Recursively inspect externally visible public API items.
///
/// # Termination
/// - reason: each descent processes one parsed inline module or resolved child
///   module file.
/// - measure: unvisited module item lists reachable from the public module
///   frontier.
/// - boundedness: syn provides finite item lists and traversal follows declared
///   modules only.
/// - input recursion: none.
fn inspect_module_items(
    inspection: &ModuleInspection<'_>,
    findings: &mut Vec<Finding>,
) -> Result<(), GateError>
{
    if !inspection.parent_public {
        return Ok(());
    }

    let mut external_modules = Vec::new();
    inspect_module_item_tree(
        inspection.workspace_root,
        PackageText(inspection.package),
        ModuleInspectionWork {
            module_dir: inspection.module_dir.to_path_buf(),
            path: inspection.path.clone(),
            items: inspection.items,
            inherited_aliases: inspection.inherited_aliases.clone(),
        },
        findings,
        &mut external_modules,
    )?;

    while let Some(external) = external_modules.pop() {
        let syntax = parse_source_file(&external.source_path)?;
        inspect_module_item_tree(
            external.workspace_root,
            external.package,
            ModuleInspectionWork {
                module_dir: external.module_dir,
                path: external.path,
                items: &syntax.items,
                inherited_aliases: external.inherited_aliases,
            },
            findings,
            &mut external_modules,
        )?;
    }

    Ok(())
}

/// Inspect one source file's inline module tree and enqueue out-of-line
/// modules.
fn inspect_module_item_tree<'context>(
    workspace_root: &'context Path,
    package: PackageText<'context>,
    initial: ModuleInspectionWork<'_>,
    findings: &mut Vec<Finding>,
    external_modules: &mut Vec<ExternalModuleWork<'context>>,
) -> Result<(), GateError>
{
    let mut work = vec![initial];
    while let Some(step) = work.pop() {
        let aliases = module_aliases(step.items, &step.inherited_aliases);
        let public_types = public_type_names(step.items);
        let current = ModuleInspection {
            workspace_root,
            package: package.0,
            module_dir: &step.module_dir,
            path: step.path.clone(),
            items: step.items,
            parent_public: true,
            inherited_aliases: &aliases,
        };

        for item in step.items {
            match *item {
                | Item::Mod(ref module) => {
                    let module_public = is_visible_api(&module.vis).into().0;
                    if let Some(child_items) = module.content.as_ref().map(|content| &content.1) {
                        if module_public {
                            work.push(ModuleInspectionWork {
                                module_dir: step.module_dir.join(module.ident.to_string()),
                                path: step.path.clone(),
                                items: child_items,
                                inherited_aliases: aliases.clone(),
                            });
                        }
                    }
                    else if module_public {
                        let child_path = resolve_module_source(&step.module_dir, module)?;
                        let child_dir = child_module_dir(&child_path)?;
                        let child_rel = relative_path(workspace_root, &child_path);
                        external_modules.push(ExternalModuleWork {
                            workspace_root,
                            package,
                            source_path: child_path,
                            module_dir: child_dir,
                            path: child_rel,
                            inherited_aliases: aliases.clone(),
                        });
                    }
                },
                | Item::Use(ref item_use) if is_visible_api(&item_use.vis).into().0 => {
                    let roots = forbidden_roots_in_use_tree(&item_use.tree, None, &aliases);
                    emit_roots(
                        &roots,
                        findings,
                        "public-graph-boundary",
                        package,
                        &step.path,
                        format!("pub use {}", use_tree_declaration(&item_use.tree)),
                        PUBLIC_API_DETAIL,
                    );
                },
                | Item::ExternCrate(ref item_extern) if is_visible_api(&item_extern.vis).into().0 =>
                {
                    let mut roots = BTreeSet::new();
                    if is_graph_stack_name(&item_extern.ident.to_string()).into().0 {
                        roots.insert(item_extern.ident.to_string());
                    }
                    emit_roots(
                        &roots,
                        findings,
                        "public-graph-boundary",
                        package,
                        &step.path,
                        format!("pub extern crate {}", item_extern.ident),
                        PUBLIC_API_DETAIL,
                    );
                },
                | Item::Fn(ref item_fn) if is_visible_api(&item_fn.vis).into().0 => {
                    let roots = roots_in_signature(&item_fn.sig, &aliases);
                    emit_roots(
                        &roots,
                        findings,
                        "public-graph-boundary",
                        package,
                        &step.path,
                        format!("pub fn {}", item_fn.sig.ident),
                        PUBLIC_API_DETAIL,
                    );
                },
                | Item::Type(ref item_type) if is_visible_api(&item_type.vis).into().0 => {
                    let mut visitor = GraphStackVisitor::new(&aliases);
                    visitor.with_type_generics(&item_type.generics, |visitor| {
                        visitor.visit_generics(&item_type.generics);
                        visitor.visit_type(&item_type.ty);
                    });
                    emit_roots(
                        &visitor.roots,
                        findings,
                        "public-graph-boundary",
                        package,
                        &step.path,
                        format!("pub type {}", item_type.ident),
                        PUBLIC_API_DETAIL,
                    );
                },
                | Item::Const(ref item_const) if is_visible_api(&item_const.vis).into().0 => {
                    let mut visitor = GraphStackVisitor::new(&aliases);
                    visitor.with_type_generics(&item_const.generics, |visitor| {
                        visitor.visit_generics(&item_const.generics);
                        visitor.visit_type(&item_const.ty);
                    });
                    emit_roots(
                        &visitor.roots,
                        findings,
                        "public-graph-boundary",
                        package,
                        &step.path,
                        format!("pub const {}", item_const.ident),
                        PUBLIC_API_DETAIL,
                    );
                },
                | Item::Static(ref item_static) if is_visible_api(&item_static.vis).into().0 => {
                    let mut visitor = GraphStackVisitor::new(&aliases);
                    visitor.visit_type(&item_static.ty);
                    emit_roots(
                        &visitor.roots,
                        findings,
                        "public-graph-boundary",
                        package,
                        &step.path,
                        format!("pub static {}", item_static.ident),
                        PUBLIC_API_DETAIL,
                    );
                },
                | Item::Struct(ref item_struct) if is_visible_api(&item_struct.vis).into().0 => {
                    let mut visitor = GraphStackVisitor::new(&aliases);
                    visitor.with_type_generics(&item_struct.generics, |visitor| {
                        visitor.visit_generics(&item_struct.generics);
                        inspect_public_fields(&item_struct.fields, visitor);
                    });
                    emit_roots(
                        &visitor.roots,
                        findings,
                        "public-graph-boundary",
                        package,
                        &step.path,
                        format!("pub struct {}", item_struct.ident),
                        PUBLIC_API_DETAIL,
                    );
                },
                | Item::Union(ref item_union) if is_visible_api(&item_union.vis).into().0 => {
                    let mut visitor = GraphStackVisitor::new(&aliases);
                    visitor.with_type_generics(&item_union.generics, |visitor| {
                        visitor.visit_generics(&item_union.generics);
                        for field in &item_union.fields.named {
                            visitor.visit_type(&field.ty);
                        }
                    });
                    emit_roots(
                        &visitor.roots,
                        findings,
                        "public-graph-boundary",
                        package,
                        &step.path,
                        format!("pub union {}", item_union.ident),
                        PUBLIC_API_DETAIL,
                    );
                },
                | Item::Enum(ref item_enum) if is_visible_api(&item_enum.vis).into().0 => {
                    let mut visitor = GraphStackVisitor::new(&aliases);
                    visitor.with_type_generics(&item_enum.generics, |visitor| {
                        visitor.visit_generics(&item_enum.generics);
                        for variant in &item_enum.variants {
                            inspect_all_variant_fields(&variant.fields, visitor);
                        }
                    });
                    emit_roots(
                        &visitor.roots,
                        findings,
                        "public-graph-boundary",
                        package,
                        &step.path,
                        format!("pub enum {}", item_enum.ident),
                        PUBLIC_API_DETAIL,
                    );
                },
                | Item::Trait(ref item_trait) if is_visible_api(&item_trait.vis).into().0 => {
                    let mut visitor = GraphStackVisitor::new(&aliases);
                    visitor.with_type_generics(&item_trait.generics, |visitor| {
                        visitor.visit_generics(&item_trait.generics);
                        for bound in &item_trait.supertraits {
                            visitor.visit_type_param_bound(bound);
                        }
                        for trait_item in &item_trait.items {
                            inspect_trait_item(trait_item, visitor);
                        }
                    });
                    emit_roots(
                        &visitor.roots,
                        findings,
                        "public-graph-boundary",
                        package,
                        &step.path,
                        format!("pub trait {}", item_trait.ident),
                        PUBLIC_API_DETAIL,
                    );
                },
                | Item::TraitAlias(ref item_trait_alias)
                    if is_visible_api(&item_trait_alias.vis).into().0 =>
                {
                    let mut visitor = GraphStackVisitor::new(&aliases);
                    visitor.with_type_generics(&item_trait_alias.generics, |visitor| {
                        visitor.visit_generics(&item_trait_alias.generics);
                        for bound in &item_trait_alias.bounds {
                            visitor.visit_type_param_bound(bound);
                        }
                    });
                    emit_roots(
                        &visitor.roots,
                        findings,
                        "public-graph-boundary",
                        package,
                        &step.path,
                        format!("pub trait {}", item_trait_alias.ident),
                        PUBLIC_API_DETAIL,
                    );
                },
                | Item::ForeignMod(ref item_foreign) => {
                    inspect_foreign_items(item_foreign, &aliases, &current, findings);
                },
                | Item::Impl(ref item_impl) => {
                    inspect_impl_item(item_impl, &aliases, &public_types, &current, findings);
                },
                | Item::Macro(ref item_macro)
                    if item_macro
                        .attrs
                        .iter()
                        .any(|attribute| attribute.path().is_ident("macro_export")) =>
                {
                    findings.push(finding(
                        "public-graph-boundary",
                        package,
                        step.path.clone(),
                        "unsupported public macro".to_owned(),
                        PUBLIC_API_DETAIL,
                    ));
                },
                | Item::Verbatim(ref tokens) => {
                    if tokens.to_string().contains("pub") {
                        findings.push(finding(
                            "public-graph-boundary",
                            package,
                            step.path.clone(),
                            "unsupported public item".to_owned(),
                            PUBLIC_API_DETAIL,
                        ));
                    }
                },
                | _ => {},
            }
        }
    }

    Ok(())
}

/// Visit public trait associated signatures and bounds.
fn inspect_trait_item(
    trait_item: &TraitItem,
    visitor: &mut GraphStackVisitor<'_>,
)
{
    match *trait_item {
        | TraitItem::Const(ref item_const) => {
            visitor.with_type_generics(&item_const.generics, |visitor| {
                visitor.visit_generics(&item_const.generics);
                visitor.visit_type(&item_const.ty);
            });
        },
        | TraitItem::Fn(ref item_fn) => visitor.visit_signature(&item_fn.sig),
        | TraitItem::Type(ref item_type) => {
            visitor.with_type_generics(&item_type.generics, |visitor| {
                visitor.visit_generics(&item_type.generics);
                for bound in &item_type.bounds {
                    visitor.visit_type_param_bound(bound);
                }
                if let Some(default_type) = item_type.default.as_ref().map(|default| &default.1) {
                    visitor.visit_type(default_type);
                }
            });
        },
        | TraitItem::Macro(ref item_macro) => visitor.visit_trait_item_macro(item_macro),
        | TraitItem::Verbatim(ref _tokens) => {},
        | _ => {},
    }
}

/// Visit public struct fields that are externally nameable.
fn inspect_public_fields(
    fields: &Fields,
    visitor: &mut GraphStackVisitor<'_>,
)
{
    for field in fields {
        if is_visible_api(&field.vis).into().0 {
            visitor.visit_type(&field.ty);
        }
    }
}

/// Visit enum or union fields that are public with the containing item.
fn inspect_all_variant_fields(
    fields: &Fields,
    visitor: &mut GraphStackVisitor<'_>,
)
{
    for field in fields {
        visitor.visit_type(&field.ty);
    }
}

/// Collect forbidden roots that appear in a function or method signature.
fn roots_in_signature(
    signature: &syn::Signature,
    aliases: &AliasContext,
) -> BTreeSet<String>
{
    let mut visitor = GraphStackVisitor::new(aliases);
    visitor.visit_signature(signature);
    visitor.roots
}

/// Collect graph-stack aliases that are in scope for one module.
fn module_aliases(
    items: &[Item],
    inherited_aliases: &AliasContext,
) -> AliasContext
{
    let mut context = inherited_aliases.clone();
    context.local_types.extend(declared_type_names(items));

    let mut changed = true;
    while changed {
        changed = false;
        for item in items {
            match *item {
                | Item::Use(ref item_use) => {
                    changed |= collect_use_aliases(&item_use.tree, None, &mut context)
                        .into()
                        .0;
                },
                | Item::ExternCrate(ref item_extern) => {
                    let root = item_extern.ident.to_string();
                    if is_graph_stack_name(&root).into().0 {
                        let local = item_extern
                            .rename
                            .as_ref()
                            .map_or_else(|| root.clone(), |rename| rename.1.to_string());
                        changed |= context.named.insert(local, root).is_none();
                    }
                },
                | Item::Type(ref item_type) => {
                    let mut visitor = GraphStackVisitor::new(&context);
                    visitor.visit_type(&item_type.ty);
                    if let Some(root) = visitor.roots.iter().next() {
                        changed |= context
                            .named
                            .insert(item_type.ident.to_string(), root.clone())
                            .is_none();
                    }
                },
                | _ => {},
            }
        }
    }
    context
}

/// Collect public type names declared in one module.
fn public_type_names(items: &[Item]) -> BTreeSet<String>
{
    items
        .iter()
        .filter_map(|item| match *item {
            | Item::Struct(ref item_struct) if is_visible_api(&item_struct.vis).into().0 => {
                Some(item_struct.ident.to_string())
            },
            | Item::Enum(ref item_enum) if is_visible_api(&item_enum.vis).into().0 => {
                Some(item_enum.ident.to_string())
            },
            | Item::Union(ref item_union) if is_visible_api(&item_union.vis).into().0 => {
                Some(item_union.ident.to_string())
            },
            | Item::Type(ref item_type) if is_visible_api(&item_type.vis).into().0 => {
                Some(item_type.ident.to_string())
            },
            | Item::Trait(ref item_trait) if is_visible_api(&item_trait.vis).into().0 => {
                Some(item_trait.ident.to_string())
            },
            | _ => None,
        })
        .collect()
}

/// Collect all local type-bearing names declared in one module.
fn declared_type_names(items: &[Item]) -> BTreeSet<String>
{
    items
        .iter()
        .filter_map(|item| match *item {
            | Item::Struct(ref item_struct) => Some(item_struct.ident.to_string()),
            | Item::Enum(ref item_enum) => Some(item_enum.ident.to_string()),
            | Item::Union(ref item_union) => Some(item_union.ident.to_string()),
            | Item::Type(ref item_type) => Some(item_type.ident.to_string()),
            | Item::Trait(ref item_trait) => Some(item_trait.ident.to_string()),
            | _ => None,
        })
        .collect()
}

/// Work item for iterative use-tree alias/root traversal.
struct UseTreeWork<'tree>
{
    /// Current parsed use-tree node.
    tree: &'tree UseTree,
    /// Resolved inherited root, if any.
    inherited_root: Option<String>,
}

/// Work item for iterative use-tree rendering.
enum UseTreeRenderFrame<'tree>
{
    /// Render this parsed use-tree node.
    Tree(&'tree UseTree),
    /// Append a static separator or delimiter.
    StaticText(&'static str),
    /// Append owned identifier text.
    OwnedText(String),
}

/// Collect local aliases introduced by a use tree.
///
/// # Termination
/// - reason: each descent processes one child node of a parsed `use` tree.
/// - measure: unvisited `UseTree` nodes below the current node.
/// - boundedness: syn stores each `use` declaration as a finite parsed tree.
/// - input recursion: none.
fn collect_use_aliases<'semantic, R>(
    tree: &UseTree,
    inherited_root: R,
    context: &mut AliasContext,
) -> impl Into<CollectUseAliasesFlag>
where
    R: Into<OptionalInheritedRootText<'semantic>>,
{
    let mut changed = false;
    let mut work = vec![UseTreeWork {
        tree,
        inherited_root: inherited_root.into().0.map(str::to_owned),
    }];
    while let Some(step) = work.pop() {
        match *step.tree {
            | UseTree::Path(ref path) => {
                let ident_text = path.ident.to_string();
                let root = step
                    .inherited_root
                    .or_else(|| context.named.get(&ident_text).cloned())
                    .unwrap_or_else(|| ident_text.clone());
                work.push(UseTreeWork {
                    tree: &path.tree,
                    inherited_root: Some(root),
                });
            },
            | UseTree::Name(ref name) => {
                let local = name.ident.to_string();
                if let Some(root) = step
                    .inherited_root
                    .filter(|root| is_graph_stack_name(root.as_str()).into().0)
                {
                    changed |= context.named.insert(local, root).is_none();
                }
                else {
                    changed |= context.local_types.insert(local);
                }
            },
            | UseTree::Rename(ref rename) => {
                let ident_text = rename.ident.to_string();
                let root = step
                    .inherited_root
                    .or_else(|| context.named.get(&ident_text).cloned())
                    .unwrap_or_else(|| ident_text.clone());
                if is_graph_stack_name(&root).into().0 {
                    changed |= context
                        .named
                        .insert(rename.rename.to_string(), root)
                        .is_none();
                }
                else {
                    changed |= context.local_types.insert(rename.rename.to_string());
                }
            },
            | UseTree::Glob(_) => {
                if let Some(root) = step
                    .inherited_root
                    .filter(|root| is_graph_stack_name(root.as_str()).into().0)
                {
                    changed |= context.forbidden_glob_roots.insert(root);
                }
            },
            | UseTree::Group(ref group) => {
                for item in group.items.iter().rev() {
                    work.push(UseTreeWork {
                        tree: item,
                        inherited_root: step.inherited_root.clone(),
                    });
                }
            },
        }
    }
    changed
}

/// Collect type generic parameter names declared by one generic parameter list.
fn type_generic_names(generics: &syn::Generics) -> BTreeSet<String>
{
    generics
        .type_params()
        .map(|type_param| type_param.ident.to_string())
        .collect()
}

/// Return whether an impl self type is externally visible in this module.
fn self_type_is_public(
    self_ty: &syn::Type,
    aliases: &AliasContext,
    public_types: &BTreeSet<String>,
    local_types: &BTreeSet<String>,
) -> impl Into<SelfTypeIsPublicFlag>
{
    match *self_ty {
        | syn::Type::Path(ref type_path) => {
            type_path
                .path
                .segments
                .last()
                .is_some_and(|segment| public_types.contains(&segment.ident.to_string()))
                || !forbidden_path_roots(&type_path.path, aliases, local_types).is_empty()
        },
        | _ => {
            let mut visitor = GraphStackVisitor::new(aliases);
            visitor.local_types.extend(local_types.iter().cloned());
            visitor.visit_type(self_ty);
            !visitor.roots.is_empty()
        },
    }
}

/// Return the forbidden roots for a Rust path.
fn forbidden_path_roots(
    path: &syn::Path,
    aliases: &AliasContext,
    local_types: &BTreeSet<String>,
) -> BTreeSet<String>
{
    let mut roots = BTreeSet::new();
    let Some(first) = path.segments.first()
    else {
        return roots;
    };
    let first_text = first.ident.to_string();
    if is_graph_stack_name(&first_text).into().0 {
        roots.insert(first_text);
    }
    else if let Some(root) = aliases.named.get(&first_text) {
        roots.insert(root.clone());
    }
    else if path.leading_colon.is_none()
        && path.segments.len() == 1
        && !aliases.local_types.contains(&first_text)
        && !local_types.contains(&first_text)
        && !is_builtin_type_name(&first_text).into().0
    {
        roots.extend(aliases.forbidden_glob_roots.iter().cloned());
    }
    roots
}

/// Return whether an unresolved single-segment path is a Rust built-in or
/// prelude type name rather than a glob-import candidate.
fn is_builtin_type_name<'semantic, N>(name: N) -> impl Into<BuiltinTypeNameFlag>
where
    N: Into<NameText<'semantic>>,
{
    let name = name.into().0;
    matches!(
        name,
        "Self"
            | "bool"
            | "char"
            | "f32"
            | "f64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "str"
            | "Option"
            | "Result"
            | "Vec"
            | "Box"
            | "String"
    )
}

/// Collect forbidden roots that appear structurally in a use tree.
fn forbidden_roots_in_use_tree<'semantic, R>(
    tree: &UseTree,
    inherited_root: R,
    aliases: &AliasContext,
) -> BTreeSet<String>
where
    R: Into<OptionalInheritedRootText<'semantic>>,
{
    let inherited_root = inherited_root.into().0;
    let mut roots = BTreeSet::new();
    collect_use_tree_roots(tree, inherited_root, aliases, &mut roots);
    roots
}

/// Recursively collect forbidden roots from a use tree.
///
/// # Termination
/// - reason: each descent processes one child node of a parsed `use` tree.
/// - measure: unvisited `UseTree` nodes below the current node.
/// - boundedness: syn stores each `use` declaration as a finite parsed tree.
/// - input recursion: none.
fn collect_use_tree_roots<'semantic, R>(
    tree: &UseTree,
    inherited_root: R,
    aliases: &AliasContext,
    roots: &mut BTreeSet<String>,
)
where
    R: Into<OptionalInheritedRootText<'semantic>>,
{
    let mut work = vec![UseTreeWork {
        tree,
        inherited_root: inherited_root.into().0.map(str::to_owned),
    }];
    while let Some(step) = work.pop() {
        match *step.tree {
            | UseTree::Path(ref path) => {
                let ident_text = path.ident.to_string();
                let root = step
                    .inherited_root
                    .or_else(|| aliases.named.get(&ident_text).cloned())
                    .unwrap_or_else(|| ident_text.clone());
                if is_graph_stack_name(&root).into().0 {
                    roots.insert(root.clone());
                }
                work.push(UseTreeWork {
                    tree: &path.tree,
                    inherited_root: Some(root),
                });
            },
            | UseTree::Name(ref name) => {
                let ident_text = name.ident.to_string();
                let root = step
                    .inherited_root
                    .or_else(|| aliases.named.get(&ident_text).cloned())
                    .unwrap_or_else(|| ident_text.clone());
                if is_graph_stack_name(&root).into().0 {
                    roots.insert(root);
                }
            },
            | UseTree::Rename(ref rename) => {
                let ident_text = rename.ident.to_string();
                let root = step
                    .inherited_root
                    .or_else(|| aliases.named.get(&ident_text).cloned())
                    .unwrap_or_else(|| ident_text.clone());
                if is_graph_stack_name(&root).into().0 {
                    roots.insert(root);
                }
            },
            | UseTree::Glob(_) => {
                if let Some(root) = step
                    .inherited_root
                    .filter(|root| is_graph_stack_name(root.as_str()).into().0)
                {
                    roots.insert(root);
                }
            },
            | UseTree::Group(ref group) => {
                for item in group.items.iter().rev() {
                    work.push(UseTreeWork {
                        tree: item,
                        inherited_root: step.inherited_root.clone(),
                    });
                }
            },
        }
    }
}

/// Append one finding for each forbidden root in a declaration.
fn emit_roots<'semantic, K, G, P, D, E>(
    roots: &BTreeSet<String>,
    findings: &mut Vec<Finding>,
    kind: K,
    package: G,
    path: P,
    declaration: D,
    detail: E,
)
where
    K: Into<KindText<'semantic>>,
    G: Into<PackageText<'semantic>>,
    P: Into<PathText<'semantic>>,
    D: Into<String>,
    E: Into<DetailText<'semantic>>,
{
    let package = package.into().0;
    let path = path.into().0;
    let declaration: String = declaration.into();
    let detail = detail.into().0;
    let kind = kind.into().0;
    for root in roots {
        findings.push(finding(
            kind,
            package,
            path.to_owned(),
            format!("{declaration}: {root}"),
            detail,
        ));
    }
}

/// Render a stable structural declaration for a use tree.
///
/// # Termination
/// - reason: each descent renders one child node of a parsed `use` tree.
/// - measure: unrendered `UseTree` nodes below the current node.
/// - boundedness: syn stores each `use` declaration as a finite parsed tree.
/// - input recursion: none.
fn use_tree_declaration(tree: &UseTree) -> String
{
    let mut output = String::new();
    let mut frames = vec![UseTreeRenderFrame::Tree(tree)];
    while let Some(frame) = frames.pop() {
        match frame {
            | UseTreeRenderFrame::Tree(tree) => match *tree {
                | UseTree::Path(ref path) => {
                    frames.push(UseTreeRenderFrame::Tree(&path.tree));
                    frames.push(UseTreeRenderFrame::StaticText("::"));
                    frames.push(UseTreeRenderFrame::OwnedText(path.ident.to_string()));
                },
                | UseTree::Name(ref name) => {
                    frames.push(UseTreeRenderFrame::OwnedText(name.ident.to_string()));
                },
                | UseTree::Rename(ref rename) => {
                    frames.push(UseTreeRenderFrame::OwnedText(rename.rename.to_string()));
                    frames.push(UseTreeRenderFrame::StaticText(" as "));
                    frames.push(UseTreeRenderFrame::OwnedText(rename.ident.to_string()));
                },
                | UseTree::Glob(_) => frames.push(UseTreeRenderFrame::StaticText("*")),
                | UseTree::Group(ref group) => {
                    frames.push(UseTreeRenderFrame::StaticText("}"));
                    for (index, item) in group.items.iter().rev().enumerate() {
                        if index > 0 {
                            frames.push(UseTreeRenderFrame::StaticText(", "));
                        }
                        frames.push(UseTreeRenderFrame::Tree(item));
                    }
                    frames.push(UseTreeRenderFrame::StaticText("{"));
                },
            },
            | UseTreeRenderFrame::StaticText(text) => output.push_str(text),
            | UseTreeRenderFrame::OwnedText(text) => output.push_str(&text),
        }
    }
    output
}

/// Compare findings by the emitted diagnostic field order.
fn finding_cmp(
    left: &Finding,
    right: &Finding,
) -> core::cmp::Ordering
{
    left.kind
        .cmp(&right.kind)
        .then_with(|| left.package.cmp(&right.package))
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.declaration.cmp(&right.declaration))
        .then_with(|| left.detail.cmp(&right.detail))
}

/// Treat only unrestricted `pub` visibility as API-visible.
fn is_visible_api(visibility: &Visibility) -> impl Into<VisibleApiFlag>
{
    matches!(visibility, Visibility::Public(_))
}

/// Parse one Rust source file with typed gate errors.
fn parse_source_file(path: &Path) -> Result<syn::File, GateError>
{
    let source = read_text(path)?;
    syn::parse_file(&source).map_err(|error| GateError::RustParse {
        path: path.to_path_buf(),
        source: error,
    })
}

/// Resolve metadata manifest paths relative to the workspace root.
fn resolved_manifest_path(
    workspace_root: &Path,
    manifest_path: &Path,
) -> PathBuf
{
    if manifest_path.is_absolute() {
        manifest_path.to_path_buf()
    }
    else {
        workspace_root.join(manifest_path)
    }
}

/// Return whether a name is one of the graph-stack crates.
fn is_graph_stack_name<'semantic, N>(name: N) -> impl Into<GraphStackNameFlag>
where
    N: Into<NameText<'semantic>>,
{
    let name = name.into().0;
    name == "petgraph" || name == "fixedbitset"
}

/// Walk a package root iteratively and return sorted Rust source files.
fn rust_source_files(root: &Path) -> Result<Vec<PathBuf>, GateError>
{
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();

    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .map_err(|error| GateError::Io {
                path: directory.clone(),
                source: error,
            })?
            .map(|entry| {
                entry
                    .map(|dir_entry| dir_entry.path())
                    .map_err(|error| GateError::Io {
                        path: directory.clone(),
                        source: error,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort();

        for path in entries {
            let metadata = fs::symlink_metadata(&path).map_err(|error| GateError::Io {
                path: path.clone(),
                source: error,
            })?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() {
                pending.push(path);
            }
            else if metadata.is_file() && path.extension() == Some(OsStr::new("rs")) {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Append findings for any graph-stack path anywhere outside `gandr-graph`.
fn outside_source_findings<'semantic, P>(
    workspace_root: &Path,
    package: P,
    source_path: &Path,
    syntax: &syn::File,
    findings: &mut Vec<Finding>,
)
where
    P: Into<PackageText<'semantic>>,
{
    let package = package.into().0;
    let aliases = module_aliases(&syntax.items, &AliasContext::default());
    let mut visitor = GraphStackVisitor::new(&aliases);
    visitor.visit_file(syntax);
    emit_roots(
        &visitor.roots,
        findings,
        "source-declaration",
        package,
        &relative_path(workspace_root, source_path),
        "graph-stack path",
        OUTSIDE_SOURCE_DETAIL,
    );
}

/// Resolve one out-of-line module source path from `#[path]` or standard
/// candidates.
fn resolve_module_source(
    module_dir: &Path,
    module: &syn::ItemMod,
) -> Result<PathBuf, GateError>
{
    let module_name = module.ident.to_string();
    if let Some(relative_path) = module_path_override(module)? {
        let override_path = module_dir.join(relative_path);
        if path_is_regular_source(&override_path).map(|value| value.into().0)? {
            return Ok(override_path);
        }
        return Err(operational(format!(
            "missing path-overridden public module source for {module_name}: {}",
            override_path.display()
        )));
    }

    let flat = module_dir.join(format!("{module_name}.rs"));
    let nested = module_dir.join(&module_name).join("mod.rs");
    let flat_exists = path_is_regular_source(&flat).map(|value| value.into().0)?;
    let nested_exists = path_is_regular_source(&nested).map(|value| value.into().0)?;

    match (flat_exists, nested_exists) {
        | (true, false) => Ok(flat),
        | (false, true) => Ok(nested),
        | (false, false) => Err(operational(format!(
            "missing public module source for {module_name} under {}",
            module_dir.display()
        ))),
        | (true, true) => Err(operational(format!(
            "ambiguous public module source for {module_name}: {} and {}",
            flat.display(),
            nested.display()
        ))),
    }
}

/// Build one stable diagnostic finding.
fn finding<'semantic, K, G, E>(
    kind: K,
    package: G,
    path: String,
    declaration: String,
    detail: E,
) -> Finding
where
    K: Into<KindText<'semantic>>,
    G: Into<PackageText<'semantic>>,
    E: Into<DetailText<'semantic>>,
{
    let package = package.into().0;
    let detail = detail.into().0;
    let kind = kind.into().0;
    Finding {
        kind: kind.to_owned(),
        package: package.to_owned(),
        path,
        declaration,
        detail: detail.to_owned(),
    }
}

/// Return a `#[path = "..."]` override for an out-of-line module.
fn module_path_override(module: &syn::ItemMod) -> Result<Option<PathBuf>, GateError>
{
    for attribute in &module.attrs {
        if !attribute.path().is_ident("path") {
            continue;
        }
        let syn::Meta::NameValue(ref name_value) = attribute.meta
        else {
            return Err(operational(format!(
                "unsupported path attribute on public module {}",
                module.ident
            )));
        };
        let syn::Expr::Lit(ref expr_lit) = name_value.value
        else {
            return Err(operational(format!(
                "unsupported path attribute value on public module {}",
                module.ident
            )));
        };
        let syn::Lit::Str(ref path_lit) = expr_lit.lit
        else {
            return Err(operational(format!(
                "unsupported path attribute literal on public module {}",
                module.ident
            )));
        };
        return Ok(Some(PathBuf::from(path_lit.value())));
    }
    Ok(None)
}

/// Return whether a module candidate is a non-symlink Rust source file.
fn path_is_regular_source(path: &Path) -> Result<impl Into<PathIsRegularSourceFlag>, GateError>
{
    match fs::symlink_metadata(path) {
        | Ok(metadata) => Ok(!metadata.file_type().is_symlink() && metadata.is_file()),
        | Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        | Err(error) => Err(GateError::Io {
            path: path.to_path_buf(),
            source: error,
        }),
    }
}

/// Return the directory used for child modules declared by a source file.
fn child_module_dir(source_path: &Path) -> Result<PathBuf, GateError>
{
    let parent = source_path.parent().ok_or_else(|| {
        operational(format!(
            "module source has no parent directory: {}",
            source_path.display()
        ))
    })?;
    if source_path.file_name() == Some(OsStr::new("mod.rs")) {
        Ok(parent.to_path_buf())
    }
    else {
        let stem = source_path.file_stem().ok_or_else(|| {
            operational(format!(
                "module source has no file stem: {}",
                source_path.display()
            ))
        })?;
        Ok(parent.join(stem))
    }
}

/// Inspect externally visible inherent and trait impl surfaces.
fn inspect_impl_item(
    item_impl: &syn::ItemImpl,
    aliases: &AliasContext,
    public_types: &BTreeSet<String>,
    inspection: &ModuleInspection<'_>,
    findings: &mut Vec<Finding>,
)
{
    let mut header_visitor = GraphStackVisitor::new(aliases);
    header_visitor.with_type_generics(&item_impl.generics, |visitor| {
        visitor.visit_generics(&item_impl.generics);
        visitor.visit_type(&item_impl.self_ty);
        if let Some(trait_path) = item_impl.trait_.as_ref().map(|trait_item| &trait_item.1) {
            visitor.visit_path(trait_path);
        }
        if let Some(where_clause) = item_impl.generics.where_clause.as_ref() {
            visitor.visit_where_clause(where_clause);
        }
    });

    let impl_type_names = type_generic_names(&item_impl.generics);
    let self_public =
        self_type_is_public(&item_impl.self_ty, aliases, public_types, &impl_type_names)
            .into()
            .0;
    if self_public {
        let mut roots = header_visitor.roots;
        let mut visitor = GraphStackVisitor::new(aliases);
        visitor.local_types.extend(impl_type_names);
        for impl_item in &item_impl.items {
            match *impl_item {
                | ImplItem::Const(ref item_const)
                    if item_impl.trait_.is_some() || is_visible_api(&item_const.vis).into().0 =>
                {
                    visitor.visit_type(&item_const.ty);
                },
                | ImplItem::Fn(ref method)
                    if item_impl.trait_.is_some() || is_visible_api(&method.vis).into().0 =>
                {
                    visitor.visit_signature(&method.sig);
                },
                | ImplItem::Type(ref item_type)
                    if item_impl.trait_.is_some() || is_visible_api(&item_type.vis).into().0 =>
                {
                    visitor.with_type_generics(&item_type.generics, |visitor| {
                        visitor.visit_generics(&item_type.generics);
                        visitor.visit_type(&item_type.ty);
                    });
                },
                | ImplItem::Macro(ref item_macro) => visitor.visit_impl_item_macro(item_macro),
                | ImplItem::Verbatim(ref _tokens) => {},
                | _ => {},
            }
        }
        roots.extend(visitor.roots);
        emit_roots(
            &roots,
            findings,
            "public-graph-boundary",
            inspection.package,
            &inspection.path,
            "impl",
            PUBLIC_API_DETAIL,
        );
    }
}

/// Read a UTF-8 text file with typed gate errors.
fn read_text(path: &Path) -> Result<String, GateError>
{
    crate::support::HOST_FILESYSTEM.read_to_string(path)
}

/// Run Cargo metadata for the workspace root.
fn cargo_metadata(workspace_root: &Path) -> Result<String, GateError>
{
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(workspace_root)
        .output()
        .map_err(|error| GateError::Io {
            path: PathBuf::from("cargo metadata"),
            source: error,
        })?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|error| {
            operational(format!("cargo metadata produced non-UTF-8 stdout: {error}"))
        })
    }
    else {
        Err(operational(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}

/// Analyzes a workspace from already captured Cargo metadata JSON.
///
/// # Contract
/// - requires: `metadata_json` has Cargo-compatible `packages` and
///   `workspace_members` fields whose member package manifests name readable
///   package roots, either absolute or relative to `workspace_root`.
/// - ensures: returns findings for forbidden direct dependencies, outside
///   graph-stack source references, and externally visible `gandr-graph` API
///   exposure through the declared library module graph.
/// - provides: graph-boundary findings from already captured Cargo metadata.
/// - fails: returns typed gate errors for invalid metadata JSON or shape,
///   unreadable source files, ambiguous or missing public module files, and
///   Rust parse failures.
/// - panics: none.
/// - intension: returned finding order is the declared projection for sorted
///   package analysis, iterative package-root walking, structural `syn`
///   traversal, and finding sort by kind, package, path, declaration, then
///   detail.
///
/// # Errors
/// Returns JSON errors for invalid metadata, operational errors for invalid
/// metadata shape, I/O errors for unreadable source files, operational errors
/// for ambiguous or missing public module files, and Rust parse errors for
/// invalid source files.
///
/// # Adequacy
/// - hypothesis: L3 only — fixture metadata separates direct dependency,
///   source-reference, public-API, inline-module, path-override,
///   wildcard-import, module-graph, alias-resolution, impl-self-nameability,
///   impl-surface, symlink-bounding, relative-manifest, owner-private,
///   Rust-parse, and finding-order decision surfaces through exact finding
///   vectors and typed error observations.
/// - witness: `graph_boundary::tests::clean_metadata_and_sources_have_no_findings`
/// - witness: `graph_boundary::tests::direct_petgraph_and_fixedbitset_dependencies_are_rejected`
/// - witness: `graph_boundary::tests::private_outside_crate_graph_stack_use_is_rejected`
/// - witness: `graph_boundary::tests::public_module_graph_controls_api_analysis`
/// - witness: `graph_boundary::tests::ambiguous_public_module_sources_are_operational_errors`
/// - witness: `graph_boundary::tests::inline_module_directories_resolve_nested_out_of_line_modules`
/// - witness: `graph_boundary::tests::path_overridden_modules_resolve_from_effective_module_directory`
/// - witness: `graph_boundary::tests::restricted_visibility_and_private_adapters_are_not_public_api`
/// - witness: `graph_boundary::tests::alias_and_impl_surfaces_are_rejected`
/// - witness: `graph_boundary::tests::private_forbidden_glob_cannot_hide_public_signature_root`
/// - witness: `graph_boundary::tests::forbidden_glob_inside_private_module_does_not_leak`
/// - witness: `graph_boundary::tests::private_self_trait_impl_is_not_public_api`
/// - witness: `graph_boundary::tests::public_self_trait_impl_is_public_api`
/// - witness: `graph_boundary::tests::additional_public_type_bearing_forms_are_rejected`
/// - witness: `graph_boundary::tests::symlinked_source_entries_are_bounded`
/// - witness: `graph_boundary::tests::relative_manifest_paths_resolve_from_workspace_root`
/// - witness: `graph_boundary::tests::rust_parse_errors_are_operational_errors`
/// - witness: `graph_boundary::tests::findings_are_deterministically_ordered`
#[inline]
pub fn analyze_workspace<'semantic, J>(
    workspace_root: &Path,
    metadata_json: J,
) -> GateResult
where
    J: Into<MetadataJsonText<'semantic>>,
{
    let metadata_json = metadata_json.into().0;
    let metadata = parse_metadata(metadata_json)?;

    let packages = workspace_packages(metadata);
    let mut findings = Vec::new();

    for package in &packages {
        direct_dependency_findings(workspace_root, package, &mut findings);
    }

    for package in &packages {
        source_findings(workspace_root, package, &mut findings)?;
    }

    findings.sort_by(finding_cmp);
    Ok(findings)
}

/// Render a path relative to the workspace root when possible.
fn relative_path(
    workspace_root: &Path,
    path: &Path,
) -> String
{
    match path.strip_prefix(workspace_root) {
        | Ok(relative) => relative.to_string_lossy().into_owned(),
        | Err(_error) => path.to_string_lossy().into_owned(),
    }
}

/// Inspect public foreign-module declarations.
fn inspect_foreign_items(
    item_foreign: &syn::ItemForeignMod,
    aliases: &AliasContext,
    inspection: &ModuleInspection<'_>,
    findings: &mut Vec<Finding>,
)
{
    for item in &item_foreign.items {
        match *item {
            | syn::ForeignItem::Fn(ref item_fn) if is_visible_api(&item_fn.vis).into().0 => {
                let roots = roots_in_signature(&item_fn.sig, aliases);
                emit_roots(
                    &roots,
                    findings,
                    "public-graph-boundary",
                    inspection.package,
                    &inspection.path,
                    format!("pub foreign fn {}", item_fn.sig.ident),
                    PUBLIC_API_DETAIL,
                );
            },
            | syn::ForeignItem::Static(ref item_static)
                if is_visible_api(&item_static.vis).into().0 =>
            {
                let mut visitor = GraphStackVisitor::new(aliases);
                visitor.visit_type(&item_static.ty);
                emit_roots(
                    &visitor.roots,
                    findings,
                    "public-graph-boundary",
                    inspection.package,
                    &inspection.path,
                    format!("pub foreign static {}", item_static.ident),
                    PUBLIC_API_DETAIL,
                );
            },
            | syn::ForeignItem::Type(ref item_type) if is_visible_api(&item_type.vis).into().0 => {
                emit_roots(
                    &BTreeSet::new(),
                    findings,
                    "public-graph-boundary",
                    inspection.package,
                    &inspection.path,
                    format!("pub foreign type {}", item_type.ident),
                    PUBLIC_API_DETAIL,
                );
            },
            | syn::ForeignItem::Macro(ref item_macro) => {
                let mut visitor = GraphStackVisitor::new(aliases);
                visitor.visit_foreign_item_macro(item_macro);
                emit_roots(
                    &visitor.roots,
                    findings,
                    "public-graph-boundary",
                    inspection.package,
                    &inspection.path,
                    "pub foreign macro",
                    PUBLIC_API_DETAIL,
                );
            },
            | syn::ForeignItem::Verbatim(ref _tokens) => {},
            | _ => {},
        }
    }
}

/// Build a stable operational error.
fn operational<D>(detail: D) -> GateError
where
    D: Into<String>,
{
    let detail = detail.into();
    GateError::Operational { detail }
}

/// Minimal Cargo metadata shape used by the boundary gate.
struct Metadata
{
    /// Packages reported by Cargo metadata.
    packages: Vec<MetadataPackage>,
    /// Cargo package IDs that belong to the current workspace.
    workspace_members: Vec<String>,
}

/// Minimal Cargo package metadata used by the boundary gate.
struct MetadataPackage
{
    /// Stable Cargo package ID.
    id: String,
    /// Cargo package name.
    name: String,
    /// Absolute or workspace-relative path to the package manifest.
    manifest_path: PathBuf,
    /// Library target source path when supplied by Cargo metadata.
    lib_src_path: Option<PathBuf>,
    /// Direct dependencies declared by this package.
    dependencies: Vec<MetadataDependency>,
}

/// Minimal dependency metadata used by the boundary gate.
#[repr(transparent)]
struct MetadataDependency
{
    /// Dependency package name as declared by Cargo metadata.
    name: String,
}

/// Inputs for one public-module inspection step.
struct ModuleInspection<'inspection>
{
    /// Workspace root used for stable diagnostic paths.
    workspace_root: &'inspection Path,
    /// Cargo package name.
    package: &'inspection str,
    /// Directory used to resolve direct child out-of-line modules.
    module_dir: &'inspection Path,
    /// Stable diagnostic path for this source file.
    path: String,
    /// Module items to inspect.
    items: &'inspection [Item],
    /// Whether the containing module is externally visible.
    parent_public: bool,
    /// Alias and glob roots inherited by this module.
    inherited_aliases: &'inspection AliasContext,
}

/// Public-module work item for iterative inspection.
struct ModuleInspectionWork<'items>
{
    /// Directory used to resolve direct child out-of-line modules.
    module_dir: PathBuf,
    /// Stable diagnostic path for this source file.
    path: String,
    /// Module items to inspect.
    items: &'items [Item],
    /// Alias and glob roots inherited by this module.
    inherited_aliases: AliasContext,
}

/// Out-of-line module queued for parsing after the current file's inline tree.
struct ExternalModuleWork<'context>
{
    /// Workspace root used for stable diagnostic paths.
    workspace_root: &'context Path,
    /// Cargo package name.
    package: PackageText<'context>,
    /// Source file to parse for this module.
    source_path: PathBuf,
    /// Directory used to resolve this module's direct children.
    module_dir: PathBuf,
    /// Stable diagnostic path for this source file.
    path: String,
    /// Alias and glob roots inherited by this module.
    inherited_aliases: AliasContext,
}

/// Explicit local aliases to graph-stack crate roots.
type AliasMap = BTreeMap<String, String>;

/// Local name-resolution context for one module.
#[derive(Clone, Default)]
struct AliasContext
{
    /// Explicit local aliases to graph-stack crate roots.
    named: AliasMap,
    /// Forbidden wildcard imports that may supply otherwise unresolved names.
    forbidden_glob_roots: BTreeSet<String>,
    /// Type-bearing names declared lexically in this module context.
    local_types: BTreeSet<String>,
}

/// `syn` visitor that records graph-stack roots in paths and use trees.
struct GraphStackVisitor<'aliases>
{
    /// Alias and glob roots visible from this syntax subtree.
    aliases: &'aliases AliasContext,
    /// Lexical type generic binders visible in the current signature surface.
    local_types: BTreeSet<String>,
    /// Forbidden roots observed while visiting a syntax subtree.
    roots: BTreeSet<String>,
}

impl<'aliases> GraphStackVisitor<'aliases>
{
    /// Build a visitor using the supplied alias roots.
    fn new(aliases: &'aliases AliasContext) -> Self
    {
        Self {
            aliases,
            local_types: BTreeSet::new(),
            roots: BTreeSet::new(),
        }
    }

    /// Visit a syntax surface with type generic binders in lexical scope.
    fn with_type_generics<V>(
        &mut self,
        generics: &syn::Generics,
        visit: V,
    )
    where
        V: FnOnce(&mut Self),
    {
        let previous_local_types = self.local_types.clone();
        self.local_types.extend(type_generic_names(generics));
        visit(self);
        self.local_types = previous_local_types;
    }
}

impl<'ast> Visit<'ast> for GraphStackVisitor<'_>
{
    /// Record forbidden roots found in a Rust path.
    fn visit_path(
        &mut self,
        i: &'ast syn::Path,
    )
    {
        self.roots
            .extend(forbidden_path_roots(i, self.aliases, &self.local_types));
        syn::visit::visit_path(self, i);
    }

    /// Visit a function-like signature with its generic binders in scope.
    fn visit_signature(
        &mut self,
        i: &'ast syn::Signature,
    )
    {
        self.with_type_generics(&i.generics, |visitor| {
            syn::visit::visit_signature(visitor, i);
        });
    }

    /// Record forbidden roots found in a Rust use tree.
    fn visit_use_tree(
        &mut self,
        i: &'ast UseTree,
    )
    {
        self.roots
            .extend(forbidden_roots_in_use_tree(i, None, self.aliases));
        syn::visit::visit_use_tree(self, i);
    }
}

#[cfg(test)]
mod tests
{
    //! Unit witnesses for graph-boundary analysis.

    use core::error::Error;
    use core::sync::atomic::AtomicUsize;
    use core::sync::atomic::Ordering;

    use super::*;

    /// Test result alias for fallible unit witnesses.
    type TestResult = Result<(), Box<dyn Error>>;

    /// Monotonic suffix source for unique temporary workspace roots.
    static NEXT_TEMP_ROOT: AtomicUsize = AtomicUsize::new(0);

    /// Temporary workspace fixture removed on drop.
    #[repr(transparent)]
    struct TestWorkspace
    {
        /// Root path of the temporary workspace fixture.
        path: PathBuf,
    }

    impl TestWorkspace
    {
        /// Create a uniquely named temporary workspace fixture.
        fn create<'semantic, N>(name: N) -> Result<Self, GateError>
        where
            N: Into<NameText<'semantic>>,
        {
            let name = name.into().0;
            let suffix = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "gandr-workflow-gates-graph-boundary-{name}-{}-{suffix}",
                std::process::id()
            ));
            crate::support::HOST_FILESYSTEM.remove_dir_if_exists(&path)?;
            crate::support::HOST_FILESYSTEM.create_dir_all(&path)?;
            Ok(Self { path })
        }

        /// Return the fixture workspace root path.
        fn path(&self) -> &Path
        {
            &self.path
        }
    }

    impl Drop for TestWorkspace
    {
        /// Remove the temporary workspace tree.
        fn drop(&mut self)
        {
            let _cleanup_result = remove_dir_if_exists(&self.path);
        }
    }

    /// Metadata parsing edge cases surface as typed gate errors.
    #[test]
    fn metadata_parsing_edges_are_typed() -> TestResult
    {
        assert!(
            matches!(
                parse_metadata("{not-json}"),
                Err(GateError::Json { source_name, .. }) if source_name == "cargo metadata"
            ),
            "invalid metadata should be a JSON error"
        );

        let metadata = parse_metadata(
            r#"{
  "packages": [
    {
      "id": "member",
      "name": "member",
      "manifest_path": "member/Cargo.toml",
      "targets": [{"kind": ["bin"], "src_path": "member/src/main.rs"}]
    },
    {
      "id": "nonmember",
      "name": "nonmember",
      "manifest_path": "nonmember/Cargo.toml",
      "dependencies": [{"name": "petgraph"}]
    }
  ],
  "workspace_members": ["member"]
}"#,
        )?;
        let packages = workspace_packages(metadata);
        let package = packages
            .first()
            .ok_or_else(|| GateError::operational("workspace package was not selected"))?;
        assert_eq!("member", package.name);
        assert!(
            package.dependencies.is_empty(),
            "omitted dependency arrays should default to empty"
        );
        assert!(
            package.lib_src_path.is_none(),
            "non-library targets should not synthesize a library path"
        );

        assert!(
            matches!(
                parse_metadata(r#"{"packages": [], "workspace_members": [1]}"#),
                Err(GateError::Operational { detail }) if detail.contains("workspace member")
            ),
            "workspace members must be strings"
        );
        Ok(())
    }

    /// Source analysis covers public API and private use edges.
    #[test]
    fn analyze_source_modes_cover_public_and_private_edges() -> TestResult
    {
        let graph_source = r#"
pub use petgraph::{Graph as PubGraph, graph::*};
pub use fixedbitset::*;
#[macro_export]
macro_rules! exported_graph_macro { () => { petgraph::Graph::<(), ()>::new() }; }
pub struct Wrapper<T: fixedbitset::FixedBitSet> {
    pub field: PubGraph<(), ()>,
}
pub enum ExposedEnum {
    Variant { graph: petgraph::Graph<(), ()> },
}
pub union ExposedUnion {
    graph: core::mem::ManuallyDrop<petgraph::Graph<(), ()>>,
}
pub type ExposedType = fixedbitset::FixedBitSet;
pub const EXPOSED_CONST: fixedbitset::FixedBitSet = fixedbitset::FixedBitSet::new();
pub static EXPOSED_STATIC: petgraph::Graph<(), ()> = loop {};
pub trait ExposedTrait: petgraph::visit::IntoNodeReferences {
    const TRAIT_CONST: fixedbitset::FixedBitSet;
    fn trait_fn(value: petgraph::Graph<(), ()>);
    type TraitType: fixedbitset::FixedBitSet;
}
pub trait ExposedAlias = fixedbitset::FixedBitSet;
unsafe extern "C" {
    pub fn foreign_fn(value: petgraph::Graph<(), ()>);
    pub static FOREIGN_STATIC: fixedbitset::FixedBitSet;
    pub type ForeignType;
    foreign_macro!(petgraph::Graph<(), ()>);
}
pub struct PublicSelf;
impl<T: fixedbitset::FixedBitSet> PublicSelf where T: petgraph::visit::IntoNodeReferences {
    pub const IMPL_CONST: petgraph::Graph<(), ()> = loop {};
    pub fn method(value: fixedbitset::FixedBitSet) {}
    pub type ImplType = petgraph::Graph<(), ()>;
    impl_macro!(fixedbitset::FixedBitSet);
}
"#;
        let graph_findings = analyze_source(Path::new("src/lib.rs"), GRAPH_PACKAGE, graph_source)?;
        let declarations = graph_findings
            .iter()
            .map(|finding| finding.declaration.as_str())
            .collect::<BTreeSet<_>>();
        for declaration in [
            "pub use petgraph::{Graph as PubGraph, graph::*}",
            "pub use fixedbitset::*",
            "pub struct Wrapper",
            "pub enum ExposedEnum",
            "pub union ExposedUnion",
            "pub type ExposedType",
            "pub const EXPOSED_CONST",
            "pub static EXPOSED_STATIC",
            "pub trait ExposedTrait",
            "pub foreign fn foreign_fn",
            "pub foreign static FOREIGN_STATIC",
            "impl",
        ] {
            assert!(
                declarations
                    .iter()
                    .any(|actual| actual.contains(declaration)),
                "expected public graph finding for `{declaration}`"
            );
        }

        let outside_findings = analyze_source(
            Path::new("src/lib.rs"),
            "consumer",
            "use petgraph::Graph;\nfn private_use(_: Graph<(), ()>) {}\n",
        )?;
        assert!(
            outside_findings.iter().any(
                |finding| finding.kind == "source-declaration" && finding.package == "consumer"
            ),
            "outside packages should reject private graph-stack references"
        );
        assert!(
            matches!(
                analyze_source(Path::new("bad.rs"), "consumer", "fn broken("),
                Err(GateError::RustParse { path, .. }) if path.as_path() == Path::new("bad.rs")
            ),
            "parse failures should retain the source path"
        );
        Ok(())
    }

    /// Module resolution and source walking report deterministic errors.
    #[test]
    fn module_resolution_and_walk_errors_are_deterministic() -> TestResult
    {
        let fixture = TestWorkspace::create("module-resolution")?;
        let public_module: syn::ItemMod = syn::parse_str("pub mod child;")?;

        assert!(
            matches!(
                resolve_module_source(fixture.path(), &public_module),
                Err(GateError::Operational { detail }) if detail.contains("missing public module source")
            ),
            "missing out-of-line modules should be operational errors"
        );

        crate::support::HOST_FILESYSTEM.write(fixture.path().join("child.rs"), "")?;
        crate::support::HOST_FILESYSTEM.create_dir_all(fixture.path().join("child"))?;
        crate::support::HOST_FILESYSTEM.write(fixture.path().join("child/mod.rs"), "")?;
        assert!(
            matches!(
                resolve_module_source(fixture.path(), &public_module),
                Err(GateError::Operational { detail }) if detail.contains("ambiguous public module source")
            ),
            "flat and nested module candidates should be ambiguous"
        );

        let missing_override: syn::ItemMod =
            syn::parse_str(r#"#[path = "missing.rs"] pub mod child;"#)?;
        assert!(
            matches!(
                resolve_module_source(fixture.path(), &missing_override),
                Err(GateError::Operational { detail }) if detail.contains("missing path-overridden public module source")
            ),
            "missing path overrides should name the override path"
        );

        for source in [
            "#[path] pub mod child;",
            r#"#[path = concat!("child", ".rs")] pub mod child;"#,
            "#[path = 1] pub mod child;",
        ] {
            let module: syn::ItemMod = syn::parse_str(source)?;
            assert!(
                matches!(
                    module_path_override(&module),
                    Err(GateError::Operational { .. })
                ),
                "unsupported path attribute shape should fail closed"
            );
        }

        let walk_root = fixture.path().join("walk");
        let nested = walk_root.join("nested");
        let linked = walk_root.join("linked");
        crate::support::HOST_FILESYSTEM.create_dir_all(&nested)?;
        crate::support::HOST_FILESYSTEM.write(walk_root.join("b.rs"), "")?;
        crate::support::HOST_FILESYSTEM.write(nested.join("a.rs"), "")?;
        symlink_directory(&nested, &linked)?;
        let walked = rust_source_files(&walk_root)?;
        assert_eq!(
            vec![walk_root.join("b.rs"), nested.join("a.rs")],
            walked,
            "source walking should sort files and skip symlinked directories"
        );

        let missing_path = fixture.path().join("missing.rs");
        assert!(!path_is_regular_source(&missing_path).map(|value| value.into().0)?);
        Ok(())
    }

    /// Alias, glob, and built-in name resolution stay explicit.
    #[test]
    fn alias_and_builtin_resolution_edges_are_explicit() -> TestResult
    {
        let file = syn::parse_file(
            r#"
use petgraph as pg;
use pg::Graph as LocalGraph;
use fixedbitset::*;
pub struct LocalGraph;
"#,
        )?;
        let aliases = module_aliases(&file.items, &AliasContext::default());
        assert_eq!(
            Some("petgraph"),
            aliases.named.get("pg").map(String::as_str)
        );
        assert_eq!(
            Some("petgraph"),
            aliases.named.get("LocalGraph").map(String::as_str)
        );
        assert!(
            aliases.forbidden_glob_roots.contains("fixedbitset"),
            "forbidden wildcard roots should be recorded"
        );
        assert!(
            aliases.local_types.contains("LocalGraph"),
            "local type declarations should shadow aliases"
        );

        let local_path: syn::Path = syn::parse_str("LocalOnly")?;
        let local_types = BTreeSet::from([String::from("LocalOnly")]);
        assert!(
            forbidden_path_roots(&local_path, &AliasContext::default(), &local_types).is_empty(),
            "explicit local type names should not be treated as graph-stack roots"
        );
        let builtin: syn::Path = syn::parse_str("Result")?;
        assert!(
            forbidden_path_roots(&builtin, &AliasContext::default(), &BTreeSet::new()).is_empty(),
            "built-in type names should not be glob candidates"
        );
        Ok(())
    }

    /// Remove a directory tree when it exists.
    fn remove_dir_if_exists(path: &Path) -> Result<(), GateError>
    {
        crate::support::HOST_FILESYSTEM.remove_dir_if_exists(path)
    }

    /// Link a directory, or create it where symlinks are unavailable.
    fn symlink_directory(
        source: &Path,
        destination: &Path,
    ) -> Result<(), GateError>
    {
        #[cfg(unix)]
        {
            return crate::support::HOST_FILESYSTEM.symlink(source, destination);
        }
        #[cfg(not(unix))]
        {
            crate::support::HOST_FILESYSTEM.create_dir(destination)
        }
    }
}
