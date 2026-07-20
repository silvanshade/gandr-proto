//! Command-line entry point for the static spec-documentation tool.
//!
//! Subcommands: `check` (parse and validate the corpus), `build` (render the
//! no-JavaScript `HTML`), and `fmt` (canonical `XML` formatting). Argument
//! parsing is explicit and allocation-light; domain modules own all side
//! effects.

use std::path::PathBuf;
use std::process::ExitCode;

use gandr_workflow_docs::DocError;
use gandr_workflow_docs::corpus;

/// Default spec directory relative to the working directory.
const DEFAULT_SPEC_DIR: &str = "docs/spec";
/// Default build output directory relative to the working directory.
const DEFAULT_OUT_DIR: &str = "target/docs-spec";
/// References-file basename inside the spec directory.
const REFS_BASENAME: &str = "refs.yml";

/// Parsed command-line options shared across subcommands.
struct Options
{
    /// Spec directory to scan for component files.
    spec: PathBuf,
    /// References file; defaults to `<spec>/refs.yml`.
    refs: Option<PathBuf>,
    /// Build output directory.
    out: PathBuf,
    /// Positional file arguments (used by `fmt`).
    files: Vec<PathBuf>,
}

impl Options
{
    /// The effective references-file path.
    fn refs_path(&self) -> PathBuf
    {
        self.refs
            .clone()
            .unwrap_or_else(|| self.spec.join(REFS_BASENAME))
    }
}

/// Process entry point.
fn main() -> ExitCode
{
    let args: Vec<String> = std::env::args().skip(1usize).collect();
    match run(&args) {
        | Ok(code) => code,
        | Err(error) => {
            eprintln!("gandr-workflow-docs: {error}");
            ExitCode::FAILURE
        },
    }
}

/// Dispatch on the subcommand.
fn run(args: &[String]) -> Result<ExitCode, DocError>
{
    let Some((command, rest)) = args.split_first()
    else {
        return Err(DocError::usage(
            "expected a subcommand: check | build | fmt",
        ));
    };
    let options = parse_options(rest)?;
    match command.as_str() {
        | "check" => run_check(&options),
        | "build" => run_build(&options),
        | "fmt" => run_fmt(&options),
        | other => Err(DocError::usage(format!(
            "unknown subcommand '{other}' (expected check | build | fmt)"
        ))),
    }
}

/// Parse the shared options from the argument tail.
fn parse_options(args: &[String]) -> Result<Options, DocError>
{
    let mut spec = PathBuf::from(DEFAULT_SPEC_DIR);
    let mut refs: Option<PathBuf> = None;
    let mut out = PathBuf::from(DEFAULT_OUT_DIR);
    let mut files = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            | "--spec" => spec = PathBuf::from(expect_value(&mut iter, "--spec")?),
            | "--refs" => refs = Some(PathBuf::from(expect_value(&mut iter, "--refs")?)),
            | "--out" => out = PathBuf::from(expect_value(&mut iter, "--out")?),
            | flag if flag.starts_with("--") => {
                return Err(DocError::usage(format!("unknown flag '{flag}'")));
            },
            | positional => files.push(PathBuf::from(positional)),
        }
    }
    Ok(Options {
        spec,
        refs,
        out,
        files,
    })
}

/// Consume the value following a flag.
fn expect_value<'value, Iter>(
    iter: &mut Iter,
    flag: &str,
) -> Result<&'value str, DocError>
where
    Iter: Iterator<Item = &'value String>,
{
    iter.next()
        .map(String::as_str)
        .ok_or_else(|| DocError::usage(format!("flag '{flag}' expects a value")))
}

/// Run the `check` subcommand.
fn run_check(options: &Options) -> Result<ExitCode, DocError>
{
    let report = corpus::check(&options.spec, &options.refs_path())?;
    if report.diagnostics.is_empty() {
        println!(
            "check: ok ({} component(s), {})",
            report.component_count,
            options.spec.display(),
        );
        return Ok(ExitCode::SUCCESS);
    }
    for diagnostic in &report.diagnostics {
        eprintln!("{diagnostic}");
    }
    eprintln!("check: {} diagnostic(s)", report.diagnostics.len());
    Ok(ExitCode::FAILURE)
}

/// Run the `build` subcommand.
fn run_build(options: &Options) -> Result<ExitCode, DocError>
{
    let report = corpus::build(&options.spec, &options.refs_path(), &options.out)?;
    if !report.check.diagnostics.is_empty() {
        for diagnostic in &report.check.diagnostics {
            eprintln!("{diagnostic}");
        }
        eprintln!(
            "build: refused, {} diagnostic(s)",
            report.check.diagnostics.len()
        );
        return Ok(ExitCode::FAILURE);
    }
    for note in &report.notes {
        eprintln!("build note: {note}");
    }
    println!(
        "build: wrote {} page(s) to {}",
        report.pages.len(),
        options.out.display(),
    );
    Ok(ExitCode::SUCCESS)
}

/// Run the `fmt` subcommand.
fn run_fmt(options: &Options) -> Result<ExitCode, DocError>
{
    let targets = if options.files.is_empty() {
        corpus::discover_component_files(&options.spec)?
    }
    else {
        options.files.clone()
    };
    let changed = corpus::format_paths(&targets)?;
    for path in &changed {
        println!("formatted {}", path.display());
    }
    Ok(ExitCode::SUCCESS)
}
