//! `gandr-workflow-docs` CLI: the prose-document-class gate and canonical
//! `XML` formatting.

#![expect(
    clippy::print_stderr,
    clippy::print_stdout,
    reason = "standard io allowed for binaries"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use gandr_workflow_docs::corpus;

/// The repository root (two levels above this crate's manifest).
const REPO_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// The Hayagriva register `cite` keys resolve against.
// The register this tool resolved `cite` keys against left the repository with
// the specification corpus; the path below no longer exists in this tree, and
// re-sourcing it is part of the parked tool's revisit.
const REFERENCES: &str = "docs/gandr/spec/bibliography.yml";

/// Entry point: `check-docs` | `fmt [FILES...]`.
fn main() -> ExitCode
{
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        | Ok(()) => ExitCode::SUCCESS,
        | Err(message) => {
            eprintln!("workflow-docs: {message}");
            ExitCode::FAILURE
        },
    }
}

/// Dispatch and run one command.
fn run(args: &[String]) -> Result<(), String>
{
    match args.first().map(String::as_str) {
        | Some("check-docs") => check_docs_lane(),
        | Some("fmt") => fmt_lane(args.get(1 ..).unwrap_or(&[])),
        | _ => Err("usage: check-docs|fmt [FILES...]".to_owned()),
    }
}

/// The `check-docs` lane body: parse=validate the prose document classes.
fn check_docs_lane() -> Result<(), String>
{
    let root = PathBuf::from(REPO_ROOT);
    let report = corpus::check_docs(&root, &root.join(REFERENCES)).map_err(|e| e.to_string())?;
    if report.diagnostics.is_empty() {
        println!("check-docs: ok ({} document(s))", report.record_count);
        return Ok(());
    }
    for diagnostic in &report.diagnostics {
        eprintln!("{diagnostic}");
    }
    Err(format!(
        "check-docs: {} diagnostic(s)",
        report.diagnostics.len()
    ))
}

/// The `fmt` lane body: canonical `XML` formatting over the given files (the
/// prose-document roots when none are named).
fn fmt_lane(files: &[String]) -> Result<(), String>
{
    let targets = if files.is_empty() {
        let root = PathBuf::from(REPO_ROOT);
        let dirs = corpus::doc_class_dirs(&root).map_err(|e| e.to_string())?;
        corpus::discover_doc_files(&dirs).map_err(|e| e.to_string())?
    }
    else {
        files.iter().map(PathBuf::from).collect()
    };
    let changed = corpus::format_paths(&targets).map_err(|e| e.to_string())?;
    for path in &changed {
        println!("formatted {}", path.display());
    }
    Ok(())
}
