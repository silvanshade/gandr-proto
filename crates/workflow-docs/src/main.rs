//! `gandr-workflow-docs` CLI: the documentation tool lanes — `GF` toolchain
//! provisioning, grammar compilation, lexicon generation, corpus validation
//! and rendering, the prose-document-class gate, and canonical `XML`
//! formatting.

use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use gandr_workflow_docs::bibliography;
use gandr_workflow_docs::corpus;
use gandr_workflow_docs::lexicon::generate;
use gandr_workflow_docs::pipeline::IndexEntry;
use gandr_workflow_docs::pipeline::PostContext;
use gandr_workflow_docs::pipeline::build_page;
use gandr_workflow_docs::pipeline::copy_fonts;
use gandr_workflow_docs::pipeline::render_index;
use gandr_workflow_docs::typst_leaf;
use gandr_workflow_grammatical_framework::rt::GfRuntime as _;
use gandr_workflow_grammatical_framework::rt::PyPgf;

/// The repository root (two levels above this crate's manifest).
const REPO_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// The GF release the toolchain lane pins.
const GF_VERSION: &str = "3.12";

/// Entry point: `toolchain` | `grammar` | `lexicon [--check]` | `check-all` |
/// `build-all --out O` | `check --pgf P --lang L --gfd G` | `build --pgf P
/// --lang L --gfd G --out O` | `check-docs` | `fmt [FILES...]`.
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
        | Some("toolchain") => toolchain(),
        | Some("grammar") => grammar(),
        | Some("check") => {
            let pgf = flag(args, "--pgf")?;
            let lang = flag(args, "--lang")?;
            let gfd = flag(args, "--gfd")?;
            do_check(&pgf, &lang.to_string_lossy(), &gfd)
        },
        | Some("build") => {
            let pgf = flag(args, "--pgf")?;
            let lang = flag(args, "--lang")?;
            let gfd = flag(args, "--gfd")?;
            let out = flag(args, "--out")?;
            do_build(&pgf, &lang.to_string_lossy(), &gfd, &out)
        },
        | Some("lexicon") => lexicon(args.iter().any(|arg| arg == "--check")),
        | Some("check-all") => check_all(),
        | Some("build-all") => {
            let out = flag(args, "--out")?;
            build_all(&out)
        },
        | Some("check-docs") => check_docs_lane(),
        | Some("fmt") => fmt_lane(&args[1 ..]),
        | _ => Err(
            "usage: toolchain|grammar|lexicon|check-all|build-all|check|build|check-docs|fmt with --pgf/--lang/--gfd/--out"
                .to_owned(),
        ),
    }
}

/// The `check-docs` lane body: parse=validate the prose document classes.
fn check_docs_lane() -> Result<(), String>
{
    let root = PathBuf::from(REPO_ROOT);
    let report =
        corpus::check_docs(&root, &root.join("docs/spec/refs.yml")).map_err(|e| e.to_string())?;
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

/// The check-all lane body: lexicon freshness, then the mandatory
/// `checkExpr` lane over every corpus `.gfd`.
fn check_all() -> Result<(), String>
{
    lexicon(true)?;
    let root = PathBuf::from(REPO_ROOT);
    let pgf = root.join("target/gf/GandrDocsLex.pgf");
    let runtime =
        PyPgf::load(&pgf.to_string_lossy(), "GandrDocsLexHtml").map_err(|e| e.to_string())?;
    for gfd in corpus_files()? {
        let text = std::fs::read_to_string(&gfd).map_err(|e| e.to_string())?;
        runtime
            .check(&text)
            .map_err(|e| format!("{}: {e}", gfd.display()))?;
        println!("checked: {}", gfd.display());
    }
    Ok(())
}

/// The build-all lane body: render every corpus page, copy the fonts, and
/// emit the corpus index page.
fn build_all(out: &Path) -> Result<(), String>
{
    let root = PathBuf::from(REPO_ROOT);
    let pgf = root.join("target/gf/GandrDocsLex.pgf");
    let runtime =
        PyPgf::load(&pgf.to_string_lossy(), "GandrDocsLexHtml").map_err(|e| e.to_string())?;
    let bibliography =
        bibliography::load(&root.join("docs/spec/refs.yml")).map_err(|e| e.to_string())?;
    let cache_dir = typst_leaf::default_cache_dir(&root.join("target/gf"));
    let context = PostContext::new(&bibliography, &cache_dir);
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for gfd in corpus_files()? {
        let text = std::fs::read_to_string(&gfd).map_err(|e| e.to_string())?;
        let stem = gfd
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned();
        let page = build_page(&runtime, &text, &stem, &context)
            .map_err(|e| format!("{}: {e}", gfd.display()))?;
        std::fs::write(out.join(format!("{stem}.html")), page).map_err(|e| e.to_string())?;
        entries.push(index_entry(&text, &stem)?);
        println!("built: {stem}.html");
    }
    copy_fonts(out).map_err(|e| e.to_string())?;
    std::fs::write(out.join("index.html"), render_index(&entries)).map_err(|e| e.to_string())?;
    println!("built: index.html");
    Ok(())
}

/// The corpus `.gfd` files, sorted.
fn corpus_files() -> Result<Vec<PathBuf>, String>
{
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let mut files = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "gfd"))
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

/// Extract the index-row data (title, status) from a component's `.gfd` tree.
fn index_entry(
    gfd: &str,
    stem: &str,
) -> Result<IndexEntry, String>
{
    use gandr_workflow_grammatical_framework::sexp::Sexp;
    let tree = gandr_workflow_grammatical_framework::sexp::parse(gfd)
        .map_err(|e| format!("{stem}: {e}"))?;
    let Sexp::App { head, args } = &tree
    else {
        return Err(format!("{stem}: corpus root is not an application"));
    };
    if head != "MkComponent" {
        return Err(format!("{stem}: corpus root is {head}, not MkComponent"));
    }
    let [_anchor, title, status, _grounds, _derives, _sections, _refs] = args.as_slice()
    else {
        return Err(format!("{stem}: MkComponent arity is not seven"));
    };
    let (Sexp::Atom(title), Sexp::Atom(status)) = (title, status)
    else {
        return Err(format!("{stem}: MkComponent title/status is not an atom"));
    };
    let title = gandr_workflow_grammatical_framework::sexp::unquote(title)
        .ok_or_else(|| format!("{stem}: MkComponent title is not a string literal"))?;
    let status = match status.as_str() {
        | "StatusBuilt" => "built",
        | "StatusPartial" => "partial",
        | "StatusAdoptedUnbuilt" => "adopted-unbuilt",
        | "StatusDesignPass" => "design-pass",
        | "StatusDormant" => "dormant",
        | other => return Err(format!("{stem}: unknown status constructor {other}")),
    };
    Ok(IndexEntry::new(stem, &title, status))
}

/// The lexicon lane body: generate the corpus-wide `GF` lexicon modules at
/// their canonical grammar paths, or verify the committed modules are fresh
/// (`--check`, the derived-file gate pattern).
fn lexicon(check: bool) -> Result<(), String>
{
    let root = PathBuf::from(REPO_ROOT);
    let corpus_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let lexicon =
        generate(&corpus_dir, &root.join("docs/spec/refs.yml")).map_err(|e| e.to_string())?;
    let grammar_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("grammar");
    for (name, text) in [
        ("GandrDocsLex.gf", lexicon.render_abstract()),
        ("GandrDocsLexHtml.gf", lexicon.render_concrete()),
    ] {
        let path = grammar_dir.join(name);
        if check {
            let committed = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            if committed != text {
                return Err(format!("{} is stale; run the lexicon lane", path.display()));
            }
            println!("lexicon fresh: {}", path.display());
        }
        else {
            std::fs::write(&path, text).map_err(|e| e.to_string())?;
            println!("lexicon generated: {}", path.display());
        }
    }
    Ok(())
}

/// The check lane body: validate one `.gfd` at the `checkExpr` lane.
fn do_check(
    pgf: &Path,
    lang: &str,
    gfd: &Path,
) -> Result<(), String>
{
    let runtime = PyPgf::load(&pgf.to_string_lossy(), lang).map_err(|e| e.to_string())?;
    let gfd = std::fs::read_to_string(gfd).map_err(|e| e.to_string())?;
    runtime.check(&gfd).map_err(|e| e.to_string())
}

/// The build lane body: validate and render one `.gfd` page.
fn do_build(
    pgf: &Path,
    lang: &str,
    gfd: &Path,
    out: &Path,
) -> Result<(), String>
{
    let runtime = PyPgf::load(&pgf.to_string_lossy(), lang).map_err(|e| e.to_string())?;
    let gfd_text = std::fs::read_to_string(gfd).map_err(|e| e.to_string())?;
    let fallback = gfd
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("gandr docs");
    let root = PathBuf::from(REPO_ROOT);
    let bibliography =
        bibliography::load(&root.join("docs/spec/refs.yml")).map_err(|e| e.to_string())?;
    let cache_dir = typst_leaf::default_cache_dir(&root.join("target/gf"));
    let context = PostContext::new(&bibliography, &cache_dir);
    let page = build_page(&runtime, &gfd_text, fallback, &context).map_err(|e| e.to_string())?;
    if let Some(dir) = out.parent() {
        copy_fonts(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(out, page).map_err(|e| e.to_string())
}

/// Provision the GF toolchain (compiler + `libpgf`) from the pinned official
/// release into `target/gf-toolchain`; idempotent.
fn toolchain() -> Result<(), String>
{
    let root = PathBuf::from(REPO_ROOT);
    let target = root.join("target");
    let (asset, ext) = release_asset()?;
    let base = target
        .join("gf-toolchain")
        .join(if ext == "pkg" { "Payload" } else { "" });
    let gf = base.join("usr/local/bin/gf");
    if gf.exists() {
        println!("gf already provisioned: {}", gf.display());
        return Ok(());
    }
    let url = format!(
        "https://github.com/GrammaticalFramework/gf-core/releases/download/release-{GF_VERSION}/{asset}"
    );
    let archive = target.join(format!("gf-release.{ext}"));
    run_command(
        "curl",
        &["-fsSL", "-o", &archive.to_string_lossy(), &url],
        &[],
    )?;
    if ext == "pkg" {
        run_command(
            "pkgutil",
            &[
                "--expand-full",
                &archive.to_string_lossy(),
                &target.join("gf-toolchain").to_string_lossy(),
            ],
            &[],
        )?;
    }
    else {
        run_command(
            "dpkg-deb",
            &[
                "-x",
                &archive.to_string_lossy(),
                &target.join("gf-toolchain").to_string_lossy(),
            ],
            &[],
        )?;
    }
    if !gf.exists() {
        return Err(format!(
            "toolchain extraction did not produce {}",
            gf.display()
        ));
    }
    let lib = base.join("usr/local/lib");
    run_command(&gf.to_string_lossy(), &["--version"], &[(
        "DYLD_FALLBACK_LIBRARY_PATH",
        lib.to_string_lossy().into_owned(),
    )])?;
    println!("gf {GF_VERSION} provisioned at {}", gf.display());
    Ok(())
}

/// The release asset for this platform.
fn release_asset() -> Result<(&'static str, &'static str), String>
{
    match (std::env::consts::OS, std::env::consts::ARCH) {
        | ("macos", "aarch64") => Ok(("gf-3.12-macos-arm.pkg", "pkg")),
        | ("linux", "x86_64") => Ok(("gf-3.12-ubuntu-24.04.deb", "deb")),
        | (os, arch) => Err(format!("no pinned GF asset for {os}/{arch}")),
    }
}

/// Compile the corpus grammar to `target/gf/GandrDocsLex.pgf`.
fn grammar() -> Result<(), String>
{
    let root = PathBuf::from(REPO_ROOT);
    let payload = root.join("target/gf-toolchain/Payload");
    let base = if payload.join("usr/local/bin/gf").exists() {
        payload
    }
    else {
        root.join("target/gf-toolchain")
    };
    let gf = base.join("usr/local/bin/gf");
    if !gf.exists() {
        return Err("gf toolchain missing — run the toolchain lane first".to_owned());
    }
    let lib = base.join("usr/local/lib");
    let out_dir = root.join("target/gf");
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let source = root.join("crates/workflow-docs/grammar/GandrDocsLexHtml.gf");
    run_command(
        &gf.to_string_lossy(),
        &[
            "--make",
            &format!("--output-dir={}", out_dir.display()),
            &source.to_string_lossy(),
        ],
        &[(
            "DYLD_FALLBACK_LIBRARY_PATH",
            lib.to_string_lossy().into_owned(),
        )],
    )?;
    if !out_dir.join("GandrDocsLex.pgf").exists() {
        return Err("grammar compilation did not produce GandrDocsLex.pgf".to_owned());
    }
    let grammar_dir = root.join("crates/workflow-docs/grammar");
    for entry in std::fs::read_dir(&grammar_dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|ext| ext == "gfo") {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    }
    println!(
        "grammar compiled: {}",
        out_dir.join("GandrDocsLex.pgf").display()
    );
    Ok(())
}

/// Run an external command, failing with its stderr on nonzero exit.
fn run_command(
    program: &str,
    arguments: &[&str],
    env: &[(&str, String)],
) -> Result<(), String>
{
    let mut command = std::process::Command::new(program);
    command
        .args(arguments)
        .current_dir(REPO_ROOT)
        .envs(env.iter().map(|&(k, ref v)| (k, v.clone())));
    let output = command.output().map_err(|e| format!("{program}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// Extract the value following a named flag.
fn flag(
    args: &[String],
    name: &str,
) -> Result<PathBuf, String>
{
    let position = args
        .iter()
        .position(|arg| arg == name)
        .ok_or_else(|| format!("missing {name}"))?;
    args.get(position.saturating_add(1))
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing value after {name}"))
}
