//! Gate embedded syntax in Rust tests onto raw string literals.
//!
//! Test fixtures and expected renderings are source material, not Rust prose.
//! This gate finds ordinary or byte string literals containing a Rust `\\n`
//! escape in test files and test modules. Authors must use a raw string for
//! embedded content, or place the narrow allow marker immediately before the
//! literal when the escape itself is the subject of the test.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

use crate::Finding;
use crate::GateError;
use crate::support::read_utf8;
use crate::support::walk_files;

/// Stable source marker for a test whose subject is escape decoding itself.
pub const ALLOW_ESCAPED_NEWLINE_MARKER: &str = "workflow-gates: allow-escaped-newline";

crate::semantic_str!(pub(crate) struct EmbeddedSourceText);
crate::semantic_bytes!(pub(crate) struct SourceBytes);
crate::semantic_copy!(pub(crate) struct TestFileFlag(bool));
crate::semantic_copy!(pub(crate) struct ByteIndex(usize));
crate::semantic_copy!(pub(crate) struct SourceLineNumber(usize));
crate::semantic_copy!(pub(crate) struct ReportFlag(bool));
crate::semantic_copy!(pub(crate) struct TestContextFlag(bool));
crate::semantic_copy!(pub(crate) struct SyntaxFlag(bool));
crate::semantic_copy!(pub(crate) struct PrefixMatchFlag(bool));

/// Run the embedded-syntax gate over all Rust sources below `workspace_root`.
///
/// This full-tree entrypoint is useful for focused audits. The merge-wall task
/// uses [`run_changed`] so existing debt does not make the incremental gate
/// permanently red.
///
/// # Contract
/// - requires: `workspace_root` contains the repository `crates/` tree.
/// - ensures: every regular or byte string containing a multiline
///   `\\\\n`-encoded syntax fixture inside a test file or test module is
///   reported unless the explicit allow marker is on the literal's line or the
///   immediately preceding line.
/// - provides: deterministic, source-only enforcement of the raw-string rule.
/// - fails: returns typed filesystem failures without suppressing findings.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError`] when the source tree or a Rust file cannot be read.
#[inline]
pub fn run(workspace_root: &Path) -> Result<Vec<Finding>, GateError>
{
    let crates_root = workspace_root.join("crates");
    let paths = walk_files(&crates_root, OsStr::new("rs"))?;
    run_paths(workspace_root, paths)
}

/// Run the gate over Rust files changed from the repository's `main` ref.
///
/// Incremental enforcement lets the wall prevent new escaped fixtures while
/// avoiding a repository-wide rewrite as part of one hygiene change.
///
/// # Errors
/// Returns [`GateError`] when Git or a changed source file cannot be read.
#[inline]
pub fn run_changed(workspace_root: &Path) -> Result<Vec<Finding>, GateError>
{
    let output = crate::support::run_output(
        OsStr::new("git"),
        &[
            OsString::from("diff"),
            OsString::from("--name-only"),
            OsString::from("--diff-filter=AM"),
            OsString::from("main...HEAD"),
        ],
        Some(workspace_root),
        true,
    )?;
    if !crate::semantic_value::<crate::support::SuccessFlag, _>(output.success()).0 {
        return Err(GateError::operational(
            "embedded-syntax: cannot enumerate files changed from main",
        ));
    }
    let paths = output
        .stdout_lossy()
        .text()
        .as_ref()
        .lines()
        .map(|relative| workspace_root.join(relative))
        .filter(|path| path.extension() == Some(OsStr::new("rs")))
        .filter(|path| path.starts_with(workspace_root.join("crates")))
        .collect::<Vec<_>>();
    run_paths(workspace_root, paths)
}

/// Analyze an ordered collection of Rust source paths.
#[inline]
fn run_paths<I>(
    workspace_root: &Path,
    paths: I,
) -> Result<Vec<Finding>, GateError>
where
    I: IntoIterator<Item = PathBuf>,
{
    let mut findings = Vec::new();
    for path in paths {
        let source = read_utf8(&path)?;
        let test_file = is_test_file(workspace_root, &path);
        findings.extend(findings_for_source(
            workspace_root,
            &path,
            EmbeddedSourceText::from(source.as_str()),
            test_file,
        ));
    }
    Ok(findings)
}

/// Analyze one Rust source file with a caller-supplied test-file
/// classification.
///
/// The scanner deliberately runs over source bytes rather than token text: it
/// can retain the source line for a stable finding and can recognize the
/// comment marker without adding a span-location dependency to the gate.
#[inline]
fn findings_for_source(
    workspace_root: &Path,
    path: &Path,
    source: EmbeddedSourceText<'_>,
    test_file: TestFileFlag,
) -> Vec<Finding>
{
    let bytes = source.as_ref().as_bytes();
    let lines = source.as_ref().lines().collect::<Vec<_>>();
    let mut scanner = Scanner {
        bytes,
        lines: &lines,
        workspace_root,
        path,
        test_file: bool::from(test_file),
        index: 0,
        line: 1,
        brace_depth: 0,
        test_contexts: Vec::new(),
        pending_test_attribute: false,
        findings: Vec::new(),
    };
    scanner.scan();
    scanner.findings
}

/// Return whether `path` is conventionally a test source file.
#[inline]
fn is_test_file(
    workspace_root: &Path,
    path: &Path,
) -> TestFileFlag
{
    let relative = path.strip_prefix(workspace_root).unwrap_or(path);
    if relative
        .components()
        .any(|component| component.as_os_str() == OsStr::new("tests"))
    {
        return TestFileFlag::from(true);
    }
    TestFileFlag::from(
        relative
            .file_stem()
            .and_then(OsStr::to_str)
            .is_some_and(|stem| stem == "tests" || stem.ends_with("_tests")),
    )
}

/// Stateful source scanner for ordinary Rust strings and test-item boundaries.
struct Scanner<'source, 'lines>
{
    /// Source bytes being scanned.
    bytes: &'source [u8],
    /// Source lines used for stable allow-marker lookup.
    lines: &'lines [&'lines str],
    /// Workspace root used to make findings repository-relative.
    workspace_root: &'source Path,
    /// Source path used in the finding.
    path: &'source Path,
    /// Whether the path is conventionally test-only.
    test_file: bool,
    /// Current byte offset in `bytes`.
    index: usize,
    /// One-based current source line.
    line: usize,
    /// Current brace nesting depth.
    brace_depth: usize,
    /// Brace depths that enclose test modules or functions.
    test_contexts: Vec<usize>,
    /// Whether the last attribute was test-related.
    pending_test_attribute: bool,
    /// Findings accumulated during the scan.
    findings: Vec<Finding>,
}

impl Scanner<'_, '_>
{
    /// Scan the complete source file without descending into comments or raw
    /// strings.
    fn scan(&mut self)
    {
        while let Some(byte) = self.bytes.get(self.index).copied() {
            if bool::from(self.starts_with(SourceBytes::from(b"//"))) {
                self.skip_line_comment();
            }
            else if bool::from(self.starts_with(SourceBytes::from(b"/*"))) {
                self.skip_block_comment();
            }
            else if byte == b'#' && self.bytes.get(self.index.saturating_add(1)) == Some(&b'[') {
                self.scan_attribute();
            }
            else if bool::from(self.starts_with(SourceBytes::from(b"br")))
                && bool::from(
                    self.looks_like_raw_string(ByteIndex::from(self.index.saturating_add(2))),
                )
            {
                self.skip_raw_string(ByteIndex::from(self.index.saturating_add(1)));
            }
            else if byte == b'r'
                && bool::from(
                    self.looks_like_raw_string(ByteIndex::from(self.index.saturating_add(1))),
                )
            {
                self.skip_raw_string(ByteIndex::from(self.index));
            }
            else if bool::from(self.starts_with(SourceBytes::from(b"b\""))) {
                self.scan_quoted(
                    ByteIndex::from(self.index.saturating_add(1)),
                    ReportFlag::from(true),
                );
            }
            else if byte == b'\"' {
                self.scan_quoted(ByteIndex::from(self.index), ReportFlag::from(true));
            }
            else if byte == b'\'' {
                self.skip_char_literal();
            }
            else {
                self.scan_code_byte();
            }
        }
    }

    /// Consume one normal code byte and maintain item-brace test context.
    fn scan_code_byte(&mut self)
    {
        let Some(byte) = self.bytes.get(self.index).copied()
        else {
            return;
        };
        match byte {
            | b'{' => {
                self.brace_depth = self.brace_depth.saturating_add(1);
                if self.pending_test_attribute {
                    self.test_contexts.push(self.brace_depth);
                    self.pending_test_attribute = false;
                }
                self.index = self.index.saturating_add(1);
            },
            | b'}' => {
                self.brace_depth = self.brace_depth.saturating_sub(1);
                self.test_contexts
                    .retain(|depth| *depth <= self.brace_depth);
                self.index = self.index.saturating_add(1);
            },
            | b';' => {
                self.pending_test_attribute = false;
                self.index = self.index.saturating_add(1);
            },
            | b'\n' => {
                self.line = self.line.saturating_add(1);
                self.index = self.index.saturating_add(1);
            },
            | _ => self.index = self.index.saturating_add(1),
        }
    }

    /// Recognize a test-related attribute and retain it until its item's body.
    fn scan_attribute(&mut self)
    {
        let start = self.index.saturating_add(2);
        let mut cursor = start;
        let mut nested = 1_usize;
        while cursor < self.bytes.len() && nested > 0 {
            let Some(byte) = self.bytes.get(cursor).copied()
            else {
                break;
            };
            match byte {
                | b'[' => nested = nested.saturating_add(1),
                | b']' => nested = nested.saturating_sub(1),
                | b'\n' => self.line = self.line.saturating_add(1),
                | _ => {},
            }
            cursor = cursor.saturating_add(1);
        }
        let attribute = self
            .bytes
            .get(start .. cursor.saturating_sub(1))
            .unwrap_or(&[]);
        let text = String::from_utf8_lossy(attribute);
        self.pending_test_attribute = text.trim() == "test"
            || text.starts_with("test(")
            || (text.contains("cfg") && text.contains("test"));
        self.index = cursor;
    }

    /// Consume a line comment, preserving the next line number.
    fn skip_line_comment(&mut self)
    {
        self.index = self.index.saturating_add(2);
        while self.index < self.bytes.len() && self.bytes.get(self.index) != Some(&b'\n') {
            self.index = self.index.saturating_add(1);
        }
    }

    /// Consume a nested block comment.
    fn skip_block_comment(&mut self)
    {
        self.index = self.index.saturating_add(2);
        let mut depth = 1_usize;
        while self.index < self.bytes.len() && depth > 0 {
            if bool::from(self.starts_with(SourceBytes::from(b"/*"))) {
                depth = depth.saturating_add(1);
                self.index = self.index.saturating_add(2);
            }
            else if bool::from(self.starts_with(SourceBytes::from(b"*/"))) {
                depth = depth.saturating_sub(1);
                self.index = self.index.saturating_add(2);
            }
            else {
                if self.bytes.get(self.index) == Some(&b'\n') {
                    self.line = self.line.saturating_add(1);
                }
                self.index = self.index.saturating_add(1);
            }
        }
    }

    /// Scan a character literal so an escaped quote cannot start a false
    /// string.
    fn skip_char_literal(&mut self)
    {
        let next = self.bytes.get(self.index.saturating_add(1)).copied();
        if next.is_none_or(|value| value.is_ascii_alphabetic()) {
            self.index = self.index.saturating_add(1);
            return;
        }
        self.index = self.index.saturating_add(1);
        let mut escaped = false;
        while self.index < self.bytes.len() {
            let Some(byte) = self.bytes.get(self.index).copied()
            else {
                break;
            };
            if byte == b'\n' {
                self.line = self.line.saturating_add(1);
            }
            self.index = self.index.saturating_add(1);
            if escaped {
                escaped = false;
            }
            else if byte == b'\\' {
                escaped = true;
            }
            else if byte == b'\'' {
                break;
            }
        }
    }

    /// Scan a regular or byte string and report an escaped newline when active.
    fn scan_quoted(
        &mut self,
        quote_index: ByteIndex,
        report: ReportFlag,
    )
    {
        let start_line = self.line;
        let mut cursor = quote_index.0.saturating_add(1);
        let mut escaped = false;
        let mut escaped_newline_count = 0_usize;
        while cursor < self.bytes.len() {
            let Some(byte) = self.bytes.get(cursor).copied()
            else {
                break;
            };
            if escaped {
                if byte == b'n' {
                    escaped_newline_count = escaped_newline_count.saturating_add(1);
                }
                escaped = false;
            }
            else if byte == b'\\' {
                escaped = true;
            }
            else if byte == b'\"' {
                cursor = cursor.saturating_add(1);
                break;
            }
            if byte == b'\n' {
                self.line = self.line.saturating_add(1);
            }
            cursor = cursor.saturating_add(1);
        }
        let content = self
            .bytes
            .get(quote_index.0.saturating_add(1) .. cursor.saturating_sub(1))
            .unwrap_or(&[]);
        if bool::from(report)
            && (escaped_newline_count >= 2 || self.line > start_line)
            && bool::from(looks_like_embedded_syntax(SourceBytes::from(content)))
            && bool::from(self.in_test_context())
        {
            self.report(SourceLineNumber::from(start_line));
        }
        self.index = cursor;
    }

    /// Consume a raw or byte-raw string without interpreting its contents.
    fn skip_raw_string(
        &mut self,
        prefix_start: ByteIndex,
    )
    {
        let mut quote = prefix_start.0;
        if self.bytes.get(quote) == Some(&b'b') {
            quote = quote.saturating_add(1);
        }
        while self.bytes.get(quote) == Some(&b'r') {
            quote = quote.saturating_add(1);
        }
        let mut hashes = 0_usize;
        while self.bytes.get(quote.saturating_add(hashes)) == Some(&b'#') {
            hashes = hashes.saturating_add(1);
        }
        quote = quote.saturating_add(hashes);
        if self.bytes.get(quote) != Some(&b'\"') {
            self.index = self.index.saturating_add(1);
            return;
        }
        let mut cursor = quote.saturating_add(1);
        while cursor < self.bytes.len() {
            let Some(byte) = self.bytes.get(cursor).copied()
            else {
                break;
            };
            if byte == b'\n' {
                self.line = self.line.saturating_add(1);
            }
            if byte == b'\"'
                && (1 ..= hashes)
                    .all(|offset| self.bytes.get(cursor.saturating_add(offset)) == Some(&b'#'))
            {
                cursor = cursor.saturating_add(hashes).saturating_add(1);
                break;
            }
            cursor = cursor.saturating_add(1);
        }
        self.index = cursor;
    }

    /// Return whether a raw-string delimiter begins at `index`.
    fn looks_like_raw_string(
        &self,
        index: ByteIndex,
    ) -> PrefixMatchFlag
    {
        let mut cursor = index.0;
        while self.bytes.get(cursor) == Some(&b'r') {
            cursor = cursor.saturating_add(1);
        }
        while self.bytes.get(cursor) == Some(&b'#') {
            cursor = cursor.saturating_add(1);
        }
        PrefixMatchFlag::from(self.bytes.get(cursor) == Some(&b'\"'))
    }

    /// Return whether the current source position lies in test code.
    #[inline]
    fn in_test_context(&self) -> TestContextFlag
    {
        TestContextFlag::from(self.test_file || !self.test_contexts.is_empty())
    }

    /// Emit one stable finding unless the explicit local allow marker is
    /// present.
    fn report(
        &mut self,
        line: SourceLineNumber,
    )
    {
        let line = line.0;
        let allowed = self
            .lines
            .get(line.saturating_sub(1))
            .is_some_and(|text| text.contains(ALLOW_ESCAPED_NEWLINE_MARKER))
            || line
                .checked_sub(2)
                .and_then(|index| self.lines.get(index))
                .is_some_and(|text| text.contains(ALLOW_ESCAPED_NEWLINE_MARKER));
        if allowed {
            return;
        }
        let relative = self
            .path
            .strip_prefix(self.workspace_root)
            .unwrap_or(self.path);
        self.findings.push(Finding::new(
            "escaped-newline-string",
            crate_name(relative),
            relative.display().to_string(),
            format!("line {line}"),
            "use a raw string for embedded content; allow only escape-decoding tests with the documented marker",
        ));
    }

    /// Return whether the current byte position starts with `prefix`.
    #[inline]
    fn starts_with(
        &self,
        prefix: SourceBytes<'_>,
    ) -> PrefixMatchFlag
    {
        PrefixMatchFlag::from(
            self.bytes
                .get(self.index .. self.index.saturating_add(prefix.0.len()))
                .is_some_and(|value| value == prefix.0),
        )
    }
}

/// Return whether a string carries recognizable gandr or surface fixture
/// syntax.
///
/// Ordinary output, diagnostic prose, and `join("\\n")` delimiters are about
/// escapes or formatting rather than embedded syntax and remain valid. The
/// markers are intentionally language-facing words and operators, so a new
/// fixture still fails closed when it resembles source material.
#[inline]
fn looks_like_embedded_syntax(bytes: SourceBytes<'_>) -> SyntaxFlag
{
    const MARKERS: &[&str] = &[
        "def ", "module ", "extern ", "perform ", "return ", "ret ", "force(", "case ", "thunk",
        "fn(", "val ", "let ", "type ", "sign ", "oper ", "data ", "rule ", "node ", "sort ", "@[",
        "#{", "#!{", "->", "<-", "=>", "Inl(", "Inr(", "F ", "U[",
    ];
    let text = String::from_utf8_lossy(bytes.0);
    SyntaxFlag::from(MARKERS.iter().any(|marker| text.contains(marker)))
}

/// Derive the owning crate name from a workspace-relative `crates/` path.
#[inline]
fn crate_name(relative: &Path) -> String
{
    let mut components = relative.components();
    while let Some(component) = components.next() {
        if component.as_os_str() == OsStr::new("crates") {
            return components
                .next()
                .map(|component| component.as_os_str().to_string_lossy().into_owned())
                .unwrap_or_default();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests
{
    use std::path::Path;

    use super::ALLOW_ESCAPED_NEWLINE_MARKER;
    use super::EmbeddedSourceText;
    use super::TestFileFlag;
    use super::findings_for_source;

    #[test]
    fn flags_regular_strings_in_test_files_and_ignores_raw_strings()
    {
        let escaped_newline = r#"\n"#;
        let source = format!(
            r###"
            const RAW: &str = r#"def x = 1;\n"#;
            #[test]
            fn fixture() {{
                let source = "def x = 1;{escaped_newline}def y = 2;{escaped_newline}";
            }}
        "###,
            escaped_newline = escaped_newline,
        );
        let findings = findings_for_source(
            Path::new("/repo"),
            Path::new("/repo/crates/demo/src/lib.rs"),
            EmbeddedSourceText::from(&source),
            TestFileFlag::from(false),
        );
        assert_eq!(1, findings.len());
        assert_eq!("escaped-newline-string", findings[0].kind);
    }

    #[test]
    fn allows_only_the_explicit_escape_subject_marker()
    {
        let source = format!(
            "#[cfg(test)]\nmod tests {{\n    // {ALLOW_ESCAPED_NEWLINE_MARKER}\n    let text = \"\\n\";\n}}\n"
        );
        let findings = findings_for_source(
            Path::new("/repo"),
            Path::new("/repo/crates/demo/src/lib.rs"),
            EmbeddedSourceText::from(&source),
            TestFileFlag::from(false),
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn scans_every_string_in_test_directory_files()
    {
        let source = "fn fixture() { let text = \"def x\\ndef y\\n\"; }";
        let findings = findings_for_source(
            Path::new("/repo"),
            Path::new("/repo/crates/demo/tests/fixture.rs"),
            EmbeddedSourceText::from(source),
            TestFileFlag::from(true),
        );
        assert_eq!(1, findings.len());
    }
}
