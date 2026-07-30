//! `gandr-workflow-docs` CLI: the documentation tool lanes — `GF` toolchain
//! provisioning, grammar compilation, lexicon generation, corpus validation
//! and rendering, the prose-document-class gate, and canonical `XML`
//! formatting.

#![expect(
    clippy::print_stderr,
    clippy::print_stdout,
    reason = "standard io allowed for binaries"
)]

extern crate alloc;

use alloc::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use gandr_workflow_docs::appgrammar;
use gandr_workflow_docs::bibliography;
use gandr_workflow_docs::corpus;
use gandr_workflow_docs::lexicon::generate;
use gandr_workflow_docs::metrics;
use gandr_workflow_docs::pipeline::IndexEntry;
use gandr_workflow_docs::pipeline::PostContext;
use gandr_workflow_docs::pipeline::build_page;
use gandr_workflow_docs::pipeline::copy_fonts;
use gandr_workflow_docs::pipeline::render_index;
use gandr_workflow_docs::pipeline::toc_entries;
use gandr_workflow_docs::typst_leaf;
use gandr_workflow_grammatical_framework::rt::ExprText;
use gandr_workflow_grammatical_framework::rt::GfRuntime as _;
use gandr_workflow_grammatical_framework::rt::LanguageName;
use gandr_workflow_grammatical_framework::rt::PyPgf;
use gandr_workflow_grammatical_framework::rt::WordText;
use gandr_workflow_grammatical_framework::sexp::Sexp;

/// The repository root (two levels above this crate's manifest).
const REPO_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// The lexicon PGF the check/build lanes validate against (repo-relative).
const LEXICON_PGF: &str = "target/gf/GandrDocsLex.pgf";

/// The concrete syntax selected for rendering.
const HTML_LANG: &str = "GandrDocsLexHtml";

/// The GF release the toolchain lane pins.
const GF_VERSION: &str = "3.12";

/// Corpus component stem used to derive one index entry.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct CorpusStem<'stem>(&'stem str);

impl<'stem> From<&'stem str> for CorpusStem<'stem>
{
    #[inline]
    fn from(stem: &'stem str) -> Self
    {
        Self(stem)
    }
}

/// Named CLI flag whose following argument is a path.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct FlagName<'name>(&'name str);

impl<'name> From<&'name str> for FlagName<'name>
{
    #[inline]
    fn from(name: &'name str) -> Self
    {
        Self(name)
    }
}

/// Requested lexicon lane behavior.
#[derive(Clone, Copy)]
enum LexiconMode
{
    /// Write generated lexicon modules.
    Write,
    /// Verify that committed lexicon modules are current.
    Check,
}

/// Platform-specific pinned GF release asset.
struct ReleaseAsset
{
    /// Asset filename published by GF.
    filename: &'static str,
    /// Archive extension and extraction discriminator.
    extension: &'static str,
}

/// One external command invocation.
#[derive(Clone, Copy)]
struct CommandSpec<'command>
{
    /// Executable name or path.
    program: &'command str,
    /// Ordered command arguments.
    arguments: &'command [&'command str],
    /// Environment additions.
    environment: &'command [(&'command str, String)],
}

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
///
/// The `GF` lanes share one runtime instance per invocation: `run` builds it
/// at the dispatch point and passes it to the phase (compositionality — the
/// phases compose against `&PyPgf`, never re-instantiate).
fn run(args: &[String]) -> Result<(), String>
{
    match args.first().map(String::as_str) {
        | Some("toolchain") => toolchain(),
        | Some("grammar") => grammar(),
        | Some("check") => {
            let pgf = flag(args, "--pgf".into())?;
            let lang = flag(args, "--lang".into())?;
            let gfd = flag(args, "--gfd".into())?;
            let runtime = PyPgf::new(&pgf, &LanguageName::new(lang.to_string_lossy().into_owned()))
                .map_err(|e| e.to_string())?;
            do_check(&runtime, &gfd)
        },
        | Some("build") => {
            let pgf = flag(args, "--pgf".into())?;
            let lang = flag(args, "--lang".into())?;
            let gfd = flag(args, "--gfd".into())?;
            let out = flag(args, "--out".into())?;
            let runtime = PyPgf::new(&pgf, &LanguageName::new(lang.to_string_lossy().into_owned()))
                .map_err(|e| e.to_string())?;
            do_build(&runtime, &gfd, &out)
        },
        | Some("lexicon") => {
            let runtime = corpus_runtime()?;
            let mode = if args.iter().any(|arg| arg == "--check") {
                LexiconMode::Check
            }
            else {
                LexiconMode::Write
            };
            lexicon(&runtime, mode)
        },
        | Some("check-all") => check_all(&corpus_runtime()?),
        | Some("build-all") => {
            let out = flag(args, "--out".into())?;
            build_all(&corpus_runtime()?, &out)
        },
        | Some("check-docs") => check_docs_lane(),
        | Some("metrics") => metrics_lane(&corpus_runtime()?, args.get(1 ..).unwrap_or(&[])),
        | Some("app-grammar") => app_grammar(&corpus_runtime()?),
        | Some("gfd-fmt") => gfd_fmt(&corpus_runtime()?, args.get(1 ..).unwrap_or(&[])),
        | Some("fmt") => fmt_lane(args.get(1 ..).unwrap_or(&[])),
        | _ => Err(
            "usage: toolchain|grammar|lexicon|check-all|build-all|check|build|check-docs|metrics|app-grammar|gfd-fmt|fmt with --pgf/--lang/--gfd/--out"
                .to_owned(),
        ),
    }
}

/// The runtime the corpus lanes validate against (the lexicon PGF and the
/// `HTML` concrete syntax).
fn corpus_runtime() -> Result<PyPgf, String>
{
    let root = PathBuf::from(REPO_ROOT);
    PyPgf::new(&root.join(LEXICON_PGF), &LanguageName::new(HTML_LANG)).map_err(|e| e.to_string())
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

/// The provisioned `gf` binary and its library directory.
fn gf_toolchain() -> Result<(PathBuf, PathBuf), String>
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
    Ok((gf, base.join("usr/local/lib")))
}

/// The `gf` driver: compile one `.gf` module to a `PGF` under `out_dir` (the
/// crate-side owner of every source→PGF compilation; the compiler boundary
/// is documented in `docs/workflow/gfd.md`).
fn gf_make(
    out_dir: &Path,
    source: &Path,
) -> Result<(), String>
{
    let (gf, lib) = gf_toolchain()?;
    run_command(CommandSpec {
        program: &gf.to_string_lossy(),
        arguments: &[
            "--make",
            &format!("--output-dir={}", out_dir.display()),
            &source.to_string_lossy(),
        ],
        environment: &[(
            "DYLD_FALLBACK_LIBRARY_PATH",
            lib.to_string_lossy().into_owned(),
        )],
    })
}

/// The app-grammar lane body: generate the domain application grammar
/// (docs-lexicon terms, proper names, corpus-seeded general supplement) and
/// compile its `PGF` (the gandr-739 build).
fn app_grammar(runtime: &PyPgf) -> Result<(), String>
{
    let root = PathBuf::from(REPO_ROOT);
    let rgl_src = root.join("target/gf-rgl/src");
    if !rgl_src.join("english/LangEng.gf").exists() {
        return Err(
            "gf-rgl clone missing — clone the pinned tag 20260403 into target/gf-rgl (gandr-739)"
                .to_owned(),
        );
    }
    let out_dir = root.join("target/gf-app");
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let corpus_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let lexicon = generate(runtime, &corpus_dir, &root.join("docs/spec/refs.yml"))
        .map_err(|e| e.to_string())?;
    let mut entries = appgrammar::term_entries(&lexicon);
    entries.extend(appgrammar::proper_phrase_entries());
    let views = metrics::LexiconViews::load(runtime).map_err(|e| e.to_string())?;
    let mut texts = Vec::new();
    for gfd in corpus_files()? {
        let text = std::fs::read_to_string(&gfd).map_err(|e| e.to_string())?;
        let tree = runtime
            .read_tree(&ExprText::new(text))
            .map_err(|e| format!("{}: {e}", gfd.display()))?;
        let paragraphs = metrics::paragraph_texts(&tree, &views)
            .map_err(|e| format!("{}: {e}", gfd.display()))?;
        texts.extend(paragraphs);
    }
    let lang_pgf = out_dir.join("Lang.pgf");
    if !lang_pgf.exists() {
        gf_make(&out_dir, &rgl_src.join("english/LangEng.gf"))?;
    }
    let lang = PyPgf::new(&lang_pgf, &LanguageName::new("LangEng")).map_err(|e| e.to_string())?;
    let mut known_cache: BTreeMap<String, bool> = BTreeMap::new();
    let supplement = appgrammar::seed_general(
        &texts,
        |word| {
            if let Some(known) = known_cache.get(word) {
                return *known;
            }
            let known = lang
                .lookup_morpho(&WordText::new(word))
                .is_ok_and(|analyses| !analyses.is_empty());
            known_cache.insert(word.to_owned(), known);
            known
        },
        100_000_usize.into(),
    );
    let supplement_count = supplement.len();
    entries.extend(supplement);
    let path = appgrammar::path_line(&rgl_src);
    let modules: [(&str, String); 4] = [
        ("GandrTermsAbs.gf", appgrammar::render_abstract(&entries)),
        ("GandrTermsEng.gf", appgrammar::render_concrete(&entries)),
        ("GandrAppLex.gf", appgrammar::render_composition_abstract()),
        (
            "GandrAppLexEng.gf",
            appgrammar::render_composition_concrete(),
        ),
    ];
    for (name, body) in modules {
        std::fs::write(out_dir.join(name), format!("{path}{body}")).map_err(|e| e.to_string())?;
    }
    gf_make(&out_dir, &out_dir.join("GandrAppLexEng.gf"))?;
    println!(
        "app grammar: {} term entries, {} seeded, {}",
        entries.len().saturating_sub(supplement_count),
        supplement_count,
        out_dir.join("GandrAppLex.pgf").display()
    );
    Ok(())
}

/// The `gfd-fmt` lane body: canonicalize one `.gfd` file's layout (read by
/// the runtime, emitted by the canonical printer — the gandr-hz8 engine).
fn gfd_fmt(
    runtime: &PyPgf,
    files: &[String],
) -> Result<(), String>
{
    for file in files {
        let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
        let tree = runtime
            .read_tree(&ExprText::new(text))
            .map_err(|e| format!("{file}: {e}"))?;
        std::fs::write(file, tree.render()).map_err(|e| format!("{file}: {e}"))?;
        println!("formatted {file}");
    }
    Ok(())
}

/// The `metrics` lane body: prose-pacing metrics over the named `.gfd`
/// files (the whole corpus when none are named).
fn metrics_lane(
    runtime: &PyPgf,
    files: &[String],
) -> Result<(), String>
{
    let views = metrics::LexiconViews::load(runtime).map_err(|e| e.to_string())?;
    let targets = if files.is_empty() {
        corpus_files()?
    }
    else {
        files.iter().map(PathBuf::from).collect::<Vec<_>>()
    };
    for path in targets {
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let tree = runtime
            .read_tree(&ExprText::new(text))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let report =
            metrics::analyze(&tree, &views).map_err(|e| format!("{}: {e}", path.display()))?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned();
        println!("== {stem}");
        print!("{}", metrics::render(&report));
    }
    Ok(())
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
fn check_all(runtime: &PyPgf) -> Result<(), String>
{
    lexicon(runtime, LexiconMode::Check)?;
    for gfd in corpus_files()? {
        let text = std::fs::read_to_string(&gfd).map_err(|e| e.to_string())?;
        runtime
            .check(&ExprText::new(text))
            .map_err(|e| format!("{}: {e}", gfd.display()))?;
        println!("checked: {}", gfd.display());
    }
    Ok(())
}

/// The build-all lane body: render every corpus page, copy the fonts, and
/// emit the corpus index page.
fn build_all(
    runtime: &PyPgf,
    out: &Path,
) -> Result<(), String>
{
    let root = PathBuf::from(REPO_ROOT);
    let bibliography =
        bibliography::load(&root.join("docs/spec/refs.yml")).map_err(|e| e.to_string())?;
    let cache_dir = typst_leaf::default_cache_dir(&root.join("target/gf"));
    let context = PostContext::new(&bibliography, &cache_dir);
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let views = metrics::LexiconViews::load(runtime).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for gfd in corpus_files()? {
        let text = std::fs::read_to_string(&gfd).map_err(|e| e.to_string())?;
        let stem = gfd
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned();
        let expr = ExprText::new(text);
        let tree = runtime
            .read_tree(&expr)
            .map_err(|e| format!("{}: {e}", gfd.display()))?;
        let toc = toc_entries(&tree, &views).map_err(|e| format!("{}: {e}", gfd.display()))?;
        let page = build_page(runtime, &expr, stem.as_str().into(), &context, &toc)
            .map_err(|e| format!("{}: {e}", gfd.display()))?;
        std::fs::write(out.join(format!("{stem}.html")), page).map_err(|e| e.to_string())?;
        entries.push(index_entry(&tree, stem.as_str().into())?);
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

/// Extract the index-row data (title, status) from a component's tree (read
/// by the runtime — the bindings-first doctrine).
fn index_entry(
    tree: &Sexp,
    stem: CorpusStem<'_>,
) -> Result<IndexEntry, String>
{
    let Sexp::App { ref head, ref args } = *tree
    else {
        return Err(format!("{}: corpus root is not an application", stem.0));
    };
    if head != "MkComponent" {
        return Err(format!(
            "{}: corpus root is {head}, not MkComponent",
            stem.0
        ));
    }
    let [
        ref _anchor,
        ref title,
        ref status,
        ref _grounds,
        ref _derives,
        ref _sections,
        ref _refs,
    ] = *args.as_slice()
    else {
        return Err(format!("{}: MkComponent arity is not seven", stem.0));
    };
    let Sexp::Atom(ref title) = *title
    else {
        return Err(format!(
            "{}: MkComponent title/status is not an atom",
            stem.0
        ));
    };
    let Sexp::Atom(ref status) = *status
    else {
        return Err(format!(
            "{}: MkComponent title/status is not an atom",
            stem.0
        ));
    };
    let title = gandr_workflow_grammatical_framework::sexp::unquote((title).into())
        .ok_or_else(|| format!("{}: MkComponent title is not a string literal", stem.0))?;
    let status = match status.as_str() {
        | "StatusBuilt" => "built",
        | "StatusPartial" => "partial",
        | "StatusAdoptedUnbuilt" => "adopted-unbuilt",
        | "StatusDesignPass" => "design-pass",
        | "StatusDormant" => "dormant",
        | other => return Err(format!("{}: unknown status constructor {other}", stem.0)),
    };
    Ok(IndexEntry::new(
        stem.0.into(),
        title.as_str().into(),
        status.into(),
    ))
}

/// The lexicon lane body: generate the corpus-wide `GF` lexicon modules at
/// their canonical grammar paths, or verify the committed modules are fresh
/// (`--check`, the derived-file gate pattern).
fn lexicon(
    runtime: &PyPgf,
    mode: LexiconMode,
) -> Result<(), String>
{
    let root = PathBuf::from(REPO_ROOT);
    let corpus_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let lexicon = generate(runtime, &corpus_dir, &root.join("docs/spec/refs.yml"))
        .map_err(|e| e.to_string())?;
    let grammar_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("grammar");
    for (name, text) in [
        ("GandrDocsLex.gf", lexicon.render_abstract()),
        ("GandrDocsLexHtml.gf", lexicon.render_concrete()),
    ] {
        let path = grammar_dir.join(name);
        if matches!(mode, LexiconMode::Check) {
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
    runtime: &PyPgf,
    gfd: &Path,
) -> Result<(), String>
{
    let gfd = std::fs::read_to_string(gfd).map_err(|e| e.to_string())?;
    runtime
        .check(&ExprText::new(gfd))
        .map_err(|e| e.to_string())
}

/// The build lane body: validate and render one `.gfd` page.
fn do_build(
    runtime: &PyPgf,
    gfd: &Path,
    out: &Path,
) -> Result<(), String>
{
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
    let page = {
        let expr = ExprText::new(gfd_text);
        let tree = runtime.read_tree(&expr).map_err(|e| e.to_string())?;
        let views = metrics::LexiconViews::load(runtime).map_err(|e| e.to_string())?;
        let toc = toc_entries(&tree, &views).map_err(|e| e.to_string())?;
        build_page(runtime, &expr, fallback.into(), &context, &toc).map_err(|e| e.to_string())?
    };
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
    let ReleaseAsset {
        filename: asset,
        extension: ext,
    } = release_asset()?;
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
    run_command(CommandSpec {
        program: "curl",
        arguments: &["-fsSL", "-o", &archive.to_string_lossy(), &url],
        environment: &[],
    })?;
    if ext == "pkg" {
        run_command(CommandSpec {
            program: "pkgutil",
            arguments: &[
                "--expand-full",
                &archive.to_string_lossy(),
                &target.join("gf-toolchain").to_string_lossy(),
            ],
            environment: &[],
        })?;
    }
    else {
        run_command(CommandSpec {
            program: "dpkg-deb",
            arguments: &[
                "-x",
                &archive.to_string_lossy(),
                &target.join("gf-toolchain").to_string_lossy(),
            ],
            environment: &[],
        })?;
    }
    if !gf.exists() {
        return Err(format!(
            "toolchain extraction did not produce {}",
            gf.display()
        ));
    }
    let lib = base.join("usr/local/lib");
    run_command(CommandSpec {
        program: &gf.to_string_lossy(),
        arguments: &["--version"],
        environment: &[(
            "DYLD_FALLBACK_LIBRARY_PATH",
            lib.to_string_lossy().into_owned(),
        )],
    })?;
    println!("gf {GF_VERSION} provisioned at {}", gf.display());
    Ok(())
}

/// The release asset for this platform.
fn release_asset() -> Result<ReleaseAsset, String>
{
    match (std::env::consts::OS, std::env::consts::ARCH) {
        | ("macos", "aarch64") => Ok(ReleaseAsset {
            filename: "gf-3.12-macos-arm.pkg",
            extension: "pkg",
        }),
        | ("linux", "x86_64") => Ok(ReleaseAsset {
            filename: "gf-3.12-ubuntu-24.04.deb",
            extension: "deb",
        }),
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
    run_command(CommandSpec {
        program: &gf.to_string_lossy(),
        arguments: &[
            "--make",
            &format!("--output-dir={}", out_dir.display()),
            &source.to_string_lossy(),
        ],
        environment: &[(
            "DYLD_FALLBACK_LIBRARY_PATH",
            lib.to_string_lossy().into_owned(),
        )],
    })?;
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
fn run_command(spec: CommandSpec<'_>) -> Result<(), String>
{
    let mut command = std::process::Command::new(spec.program);
    command.args(spec.arguments).current_dir(REPO_ROOT).envs(
        spec.environment
            .iter()
            .map(|&(key, ref value)| (key, value.clone())),
    );
    let output = command
        .output()
        .map_err(|error| format!("{}: {error}", spec.program))?;
    if !output.status.success() {
        return Err(format!(
            "{} exited {}: {}",
            spec.program,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// Extract the value following a named flag.
fn flag(
    args: &[String],
    name: FlagName<'_>,
) -> Result<PathBuf, String>
{
    let position = args
        .iter()
        .position(|arg| arg == name.0)
        .ok_or_else(|| format!("missing {}", name.0))?;
    args.get(position.saturating_add(1))
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing value after {}", name.0))
}
