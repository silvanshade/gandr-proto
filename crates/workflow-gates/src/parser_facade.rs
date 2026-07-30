//! Bounded parser-only facade shared by CLI parsing and AFL fuzzing.
//!
//! The fuzzing surface exercises parser and validator seams only. It never
//! reads repository files, writes outputs, inspects environment variables, or
//! spawns commands.
//!
//! # Contract
//! - requires: callers pass already-captured bytes or command-name tokens.
//! - ensures: parser-facade fuzzing is bounded, deterministic,
//!   side-effect-free, and honest about exercising parser/validator seams
//!   rather than execution paths.
//! - provides: shared top-level command-token parsing and the AFL parser
//!   facade.
//! - fails: unsupported command tokens surface as usage errors; fuzz parser
//!   failures are discarded after exercising their parser paths.
//! - panics: none.
//! - intension: parser domains run in a fixed order with fixed byte and token
//!   bounds.
//!
//! # Adequacy
//! - hypothesis: L3 only — command inventory, arbitrary byte, UTF-8 guard,
//!   oversize guard, and representative valid-record witnesses cover the facade
//!   decisions without claiming command execution coverage.
//! - witness: `parser_facade::tests::cli_command_tokens_cover_the_retained_inventory`
//! - witness: `parser_facade::tests::arbitrary_bytes_never_escape_the_facade`
//! - witness: `parser_facade::tests::invalid_utf8_and_oversized_inputs_skip_parser_dispatch`
//! - witness: `parser_facade::tests::representative_valid_records_reach_each_parser_domain`

extern crate alloc;

#[cfg(feature = "fuzzing")]
use alloc::borrow::Cow;
use alloc::format;
#[cfg(feature = "fuzzing")]
use alloc::vec;
#[cfg(feature = "fuzzing")]
use std::path::Path;

use crate::GateError;

crate::semantic_str!(pub struct CommitTextText);
crate::semantic_str!(pub struct TimestampTextText);
crate::semantic_str!(pub struct TextText);
crate::semantic_copy!(pub struct IndexIndex(usize));
crate::semantic_str!(pub struct ValueText);
crate::semantic_bytes!(pub struct DataBytes);
crate::semantic_str!(pub struct FirstTokenOrEmptyText);
crate::semantic_optional_str!(pub struct OptionalTokenAtText);

/// Top-level `gandr-workflow-gates` command token.
///
/// # Contract
/// - requires: `value` is the command-name token after the executable name.
/// - ensures: only retained top-level command names are representable.
/// - provides: the pure CLI command discriminator shared by the binary parser
///   and the parser-only fuzz facade.
/// - fails: returns [`GateError::Usage`] for unsupported tokens.
/// - panics: none.
///
/// # Errors
/// Returns a usage error when `value` is not one of the retained command names.
///
/// # Adequacy
/// - hypothesis: L3 only — the unit witnesses enumerate every accepted command
///   and one rejected command, while CLI integration tests still observe the
///   delegated command-specific parsers.
/// - witness: `parser_facade::tests::cli_command_tokens_cover_the_retained_inventory`
/// - witness: `tooling::top_level_command_inventory_is_exact`
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CliCommandName
{
    /// Contract-documentation adequacy gate.
    Contracts,
    /// GitHub Actions workflow contract gate.
    CiContracts,
    /// Cargo dependency graph boundary gate.
    GraphBoundary,
    /// Documentation manifest drift gate.
    DocsManifest,
    /// Documentation reference integrity gate.
    DocsReference,
    /// Typst page-balance probe gate.
    PageBalance,
    /// Guarded rumdl wrapper gate.
    Rumdl,
    /// Agda OPTIONS source policy gate.
    OptionsPolicy,
    /// Rust soundness-oracle source policy gate.
    SoundnessOracles,
    /// Default dependency-graph policy gate.
    DefaultGraph,
    /// Internal-univalence submodule pin gate.
    IuPin,
    /// Coverage floor policy gate.
    Coverage,
    /// Weekly maintenance range gate.
    MaintenanceRange,
    /// Mutation-campaign facade.
    Mutants,
    /// Local workflow tier gate.
    Workflow,
    /// Deterministic AFL smoke gate.
    FuzzSmoke,
}

impl CliCommandName
{
    /// Parse a top-level command token.
    ///
    /// # Contract
    /// - requires: `value` is a UTF-8 command token.
    /// - ensures: returns the exact retained command variant for supported
    ///   spellings.
    /// - provides: shared top-level CLI grammar without any side effects.
    /// - fails: returns a stable usage error for unsupported spellings.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Usage`] when `value` is not a supported command.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — exhaustive accepted-token and rejected-token
    ///   witnesses kill misspelled, missing, or extra command mappings.
    /// - witness: `parser_facade::tests::cli_command_tokens_cover_the_retained_inventory`
    #[inline]
    pub fn parse<'semantic, Value>(value: Value) -> Result<Self, GateError>
    where
        Value: Into<ValueText<'semantic>>,
    {
        let value = value.into().0;
        match value {
            | "contracts" => Ok(Self::Contracts),
            | "ci-contracts" => Ok(Self::CiContracts),
            | "graph-boundary" => Ok(Self::GraphBoundary),
            | "docs-manifest" => Ok(Self::DocsManifest),
            | "docs-reference" => Ok(Self::DocsReference),
            | "page-balance" => Ok(Self::PageBalance),
            | "rumdl" => Ok(Self::Rumdl),
            | "options-policy" => Ok(Self::OptionsPolicy),
            | "soundness-oracles" => Ok(Self::SoundnessOracles),
            | "default-graph" => Ok(Self::DefaultGraph),
            | "iu-pin" => Ok(Self::IuPin),
            | "coverage" => Ok(Self::Coverage),
            | "maintenance-range" => Ok(Self::MaintenanceRange),
            | "mutants" => Ok(Self::Mutants),
            | "workflow" => Ok(Self::Workflow),
            | "fuzz-smoke" => Ok(Self::FuzzSmoke),
            | other => Err(GateError::usage(format!("unknown command `{other}`"))),
        }
    }

    /// Return the stable CLI spelling for this command.
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> ValueText<'static>
    {
        match self {
            | Self::Contracts => ValueText("contracts"),
            | Self::CiContracts => ValueText("ci-contracts"),
            | Self::GraphBoundary => ValueText("graph-boundary"),
            | Self::DocsManifest => ValueText("docs-manifest"),
            | Self::DocsReference => ValueText("docs-reference"),
            | Self::PageBalance => ValueText("page-balance"),
            | Self::Rumdl => ValueText("rumdl"),
            | Self::OptionsPolicy => ValueText("options-policy"),
            | Self::SoundnessOracles => ValueText("soundness-oracles"),
            | Self::DefaultGraph => ValueText("default-graph"),
            | Self::IuPin => ValueText("iu-pin"),
            | Self::Coverage => ValueText("coverage"),
            | Self::MaintenanceRange => ValueText("maintenance-range"),
            | Self::Mutants => ValueText("mutants"),
            | Self::Workflow => ValueText("workflow"),
            | Self::FuzzSmoke => ValueText("fuzz-smoke"),
        }
    }
}

/// Maximum AFL payload bytes accepted by the parser facade.
#[cfg(feature = "fuzzing")]
const MAX_FUZZ_INPUT_BYTES: usize = 4_096;

/// Maximum whitespace tokens scanned for token-oriented parser seams.
#[cfg(feature = "fuzzing")]
const MAX_FUZZ_TOKEN_SCAN: usize = 16;

/// Exercise side-effect-free parsers and validators on arbitrary AFL input.
///
/// # Contract
/// - requires: none; `data` may contain any byte sequence.
/// - ensures: inputs larger than [`MAX_FUZZ_INPUT_BYTES`] or invalid UTF-8 are
///   discarded before parser dispatch; bounded UTF-8 inputs exercise only pure
///   parser and validator seams.
/// - provides: the public AFL entry point without filesystem, process,
///   environment, repository, or output effects.
/// - fails: never reports parser errors to the caller; all outcomes are kept
///   only as coverage for parser branches.
/// - panics: none.
/// - intension: attempts representative CLI, docs, source, coverage, mutation,
///   and maintenance parser domains in a fixed order.
///
/// # Adequacy
/// - hypothesis: L3 only — unit/property witnesses cover arbitrary bytes,
///   invalid UTF-8, empty input, oversized input, and representative valid
///   records for every parser domain.
/// - witness: `parser_facade::tests::arbitrary_bytes_never_escape_the_facade`
/// - witness: `parser_facade::tests::invalid_utf8_and_oversized_inputs_skip_parser_dispatch`
/// - witness: `parser_facade::tests::empty_input_still_exercises_utf8_safe_domains`
/// - witness: `parser_facade::tests::representative_valid_records_reach_each_parser_domain`
#[cfg(feature = "fuzzing")]
#[inline]
pub fn exercise_fuzz_input<'semantic, Data>(data: Data)
where
    Data: Into<DataBytes<'semantic>>,
{
    let data = data.into().0;
    let _report = exercise_fuzz_bytes(data);
}

/// High-level input disposition selected before parser-domain dispatch.
#[cfg(feature = "fuzzing")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputDisposition
{
    /// Bytes were not valid UTF-8, so no text parser was attempted.
    NonUtf8,
    /// Bytes exceeded the facade's fixed allocation bound.
    Oversized,
    /// Bytes were bounded UTF-8 and every parser domain was attempted.
    Exercised,
}

/// Per-domain parser outcome retained for unit witnesses.
#[cfg(feature = "fuzzing")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParserDomainStatus
{
    /// The domain was not reached because the input failed a facade guard.
    Skipped,
    /// The parser rejected the bounded text or token candidate.
    Rejected,
    /// The parser or validator completed on the bounded text or token
    /// candidate.
    Completed,
}

/// Parser-domain outcomes for one bounded UTF-8 fuzz input.
#[cfg(feature = "fuzzing")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ParserDomainReport
{
    /// Top-level CLI command-token grammar outcome.
    cli: ParserDomainStatus,
    /// Documentation command parser outcome.
    docs: ParserDomainStatus,
    /// Agda source OPTIONS analyzer outcome.
    source_options: ParserDomainStatus,
    /// Rust soundness-oracle source parser outcome.
    source_soundness: ParserDomainStatus,
    /// Coverage floor-policy parser and validator outcome.
    coverage: ParserDomainStatus,
    /// Mutation range/ref parser and validator outcome.
    mutation: ParserDomainStatus,
    /// Maintenance ref/watermark parser and validator outcome.
    maintenance: ParserDomainStatus,
}

#[cfg(feature = "fuzzing")]
impl ParserDomainReport
{
    /// Build the all-skipped report used by facade guard exits.
    #[inline]
    const fn skipped() -> Self
    {
        Self {
            cli: ParserDomainStatus::Skipped,
            docs: ParserDomainStatus::Skipped,
            source_options: ParserDomainStatus::Skipped,
            source_soundness: ParserDomainStatus::Skipped,
            coverage: ParserDomainStatus::Skipped,
            mutation: ParserDomainStatus::Skipped,
            maintenance: ParserDomainStatus::Skipped,
        }
    }
}

/// Complete parser-facade report for one AFL input.
#[cfg(feature = "fuzzing")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FuzzExerciseReport
{
    /// Input guard outcome.
    input: InputDisposition,
    /// Per-domain parser outcomes.
    domains: ParserDomainReport,
}

#[cfg(feature = "fuzzing")]
impl FuzzExerciseReport
{
    /// Build a report for an input rejected before parser dispatch.
    #[inline]
    const fn skipped(input: InputDisposition) -> Self
    {
        Self {
            input,
            domains: ParserDomainReport::skipped(),
        }
    }
}

/// Exercise parsers after enforcing byte and UTF-8 bounds.
#[cfg(feature = "fuzzing")]
fn exercise_fuzz_bytes<'semantic, Data>(data: Data) -> FuzzExerciseReport
where
    Data: Into<DataBytes<'semantic>>,
{
    let data = data.into().0;
    if data.len() > MAX_FUZZ_INPUT_BYTES {
        return FuzzExerciseReport::skipped(InputDisposition::Oversized);
    }
    let Ok(text) = core::str::from_utf8(data)
    else {
        return FuzzExerciseReport::skipped(InputDisposition::NonUtf8);
    };
    exercise_fuzz_text(text)
}

/// Exercise every parser domain on bounded UTF-8 text.
#[cfg(feature = "fuzzing")]
fn exercise_fuzz_text<'semantic, Text>(text: Text) -> FuzzExerciseReport
where
    Text: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    FuzzExerciseReport {
        input: InputDisposition::Exercised,
        domains: ParserDomainReport {
            cli: exercise_cli_text(text),
            docs: exercise_docs_text(text),
            source_options: exercise_source_options_text(text),
            source_soundness: exercise_source_soundness_text(text),
            coverage: exercise_coverage_text(text),
            mutation: exercise_mutation_text(text),
            maintenance: exercise_maintenance_text(text),
        },
    }
}

/// Exercise top-level command-token parsing over bounded whitespace tokens.
#[cfg(feature = "fuzzing")]
fn exercise_cli_text<'semantic, Text>(text: Text) -> ParserDomainStatus
where
    Text: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    let mut saw_token = false;
    for token in text.split_whitespace().take(MAX_FUZZ_TOKEN_SCAN) {
        saw_token = true;
        if CliCommandName::parse(token).is_ok() {
            return ParserDomainStatus::Completed;
        }
    }
    if !saw_token {
        let _rejected = CliCommandName::parse("");
    }
    ParserDomainStatus::Rejected
}

/// Exercise documentation command parsing over the first bounded token.
#[cfg(feature = "fuzzing")]
fn exercise_docs_text<'semantic, Text>(text: Text) -> ParserDomainStatus
where
    Text: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    let token = first_token_or_empty(text).into().0;
    result_to_status(crate::docs::commands::RumdlMode::parse(token))
}

/// Exercise pure Agda OPTIONS policy analysis over one synthetic module.
#[cfg(feature = "fuzzing")]
fn exercise_source_options_text<'semantic, Text>(text: Text) -> ParserDomainStatus
where
    Text: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    let roots = vec![crate::source_policy::OptionsRoot {
        root: Cow::Borrowed("fuzz-root"),
        modules: vec![crate::source_policy::OptionsModule {
            relative_path: Cow::Borrowed("Fuzz.agda"),
            source: Some(Cow::Borrowed(text)),
        }],
    }];
    let _findings = crate::source_policy::analyze_options_policy(
        &roots,
        &crate::source_policy::DEFAULT_OPTIONS_POLICIES,
    );
    ParserDomainStatus::Completed
}

/// Exercise Rust source parsing and soundness-oracle validation.
#[cfg(feature = "fuzzing")]
fn exercise_source_soundness_text<'semantic, Text>(text: Text) -> ParserDomainStatus
where
    Text: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    result_to_status(crate::source_policy::analyze_soundness_source(
        Path::new("fuzz-input.rs"),
        text,
    ))
}

/// Exercise coverage floor-policy parsing and validation.
#[cfg(feature = "fuzzing")]
fn exercise_coverage_text<'semantic, Text>(text: Text) -> ParserDomainStatus
where
    Text: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    result_to_status(crate::coverage::parse_floors_text_for_fuzzing(
        "afl-input.toml",
        text,
    ))
}

/// Exercise mutation range and ref-token parser seams.
#[cfg(feature = "fuzzing")]
fn exercise_mutation_text<'semantic, Text>(text: Text) -> ParserDomainStatus
where
    Text: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    let first = first_token_or_empty(text).into().0;
    let second = token_at(text, 1).into().0.unwrap_or("");
    let push_status = if second.is_empty() {
        result_to_status(crate::mutants::range::PushRangePlan::last(first))
    }
    else {
        result_to_status(crate::mutants::range::PushRangePlan::range(first, second))
    };
    status_any_completed(&[
        result_to_status(crate::mutants::range::validate_scheduled_ref_token(
            first, "from",
        )),
        result_to_status(crate::mutants::range::CommitId::parse(first, "from")),
        push_status,
    ])
}

/// Exercise maintenance ref, timestamp, base-source, and watermark parsers.
#[cfg(feature = "fuzzing")]
fn exercise_maintenance_text<'semantic, Text>(text: Text) -> ParserDomainStatus
where
    Text: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    let first = first_token_or_empty(text).into().0;
    status_any_completed(&[
        result_to_status(crate::maintenance::GitRef::new(first)),
        result_to_status(crate::maintenance::CommitId::new(first)),
        exercise_maintenance_timestamp(first, text),
        result_to_status(crate::maintenance::plan_base_source(Some(text), None)),
        result_to_status(crate::maintenance::parse_optional_watermark_text(
            "afl-watermark.json",
            Some(text),
        )),
    ])
}

/// Exercise maintenance timestamp parsing when a candidate commit is valid.
#[cfg(feature = "fuzzing")]
fn exercise_maintenance_timestamp<'semantic, Commit, Timestamp>(
    commit_text: Commit,
    timestamp_text: Timestamp,
) -> ParserDomainStatus
where
    Commit: Into<CommitTextText<'semantic>>,
    Timestamp: Into<TimestampTextText<'semantic>>,
{
    let timestamp_text = timestamp_text.into().0;
    let commit_text = commit_text.into().0;
    match crate::maintenance::CommitId::new(commit_text) {
        | Ok(commit) => result_to_status(crate::maintenance::CommitTimestamp::parse_git_output(
            &commit,
            timestamp_text,
        )),
        | Err(_error) => ParserDomainStatus::Rejected,
    }
}

/// Return the first bounded whitespace token, or an empty token when absent.
#[cfg(feature = "fuzzing")]
fn first_token_or_empty<'semantic, Text>(text: Text) -> impl Into<FirstTokenOrEmptyText<'semantic>>
where
    Text: Into<TextText<'semantic>>,
{
    token_at(text, 0).into().0.unwrap_or("")
}

/// Return the `index`th token within the bounded token scan window.
#[cfg(feature = "fuzzing")]
fn token_at<'semantic, Text, Index>(
    text: Text,
    index: Index,
) -> impl Into<OptionalTokenAtText<'semantic>>
where
    Text: Into<TextText<'semantic>>,
    Index: Into<IndexIndex>,
{
    let index = index.into().0;
    let text = text.into().0;
    text.split_whitespace().take(MAX_FUZZ_TOKEN_SCAN).nth(index)
}

/// Convert a parser result into the report status projection.
#[cfg(feature = "fuzzing")]
fn result_to_status<Value, Error>(result: Result<Value, Error>) -> ParserDomainStatus
{
    match result {
        | Ok(_value) => ParserDomainStatus::Completed,
        | Err(_error) => ParserDomainStatus::Rejected,
    }
}

/// Return completed when any candidate parser branch completed.
#[cfg(feature = "fuzzing")]
fn status_any_completed(statuses: &[ParserDomainStatus]) -> ParserDomainStatus
{
    for status in statuses {
        if *status == ParserDomainStatus::Completed {
            return ParserDomainStatus::Completed;
        }
    }
    ParserDomainStatus::Rejected
}

#[cfg(test)]
mod tests
{
    //! Unit witnesses for the parser-only fuzz facade.

    #[cfg(feature = "fuzzing")]
    use alloc::vec::Vec;

    #[cfg(feature = "fuzzing")]
    use proptest::prelude::*;

    use super::*;

    /// Inventory of retained top-level command spellings and variants.
    const COMMAND_INVENTORY: &[(CliCommandName, &str)] = &[
        (CliCommandName::Contracts, "contracts"),
        (CliCommandName::CiContracts, "ci-contracts"),
        (CliCommandName::GraphBoundary, "graph-boundary"),
        (CliCommandName::DocsManifest, "docs-manifest"),
        (CliCommandName::DocsReference, "docs-reference"),
        (CliCommandName::PageBalance, "page-balance"),
        (CliCommandName::Rumdl, "rumdl"),
        (CliCommandName::OptionsPolicy, "options-policy"),
        (CliCommandName::SoundnessOracles, "soundness-oracles"),
        (CliCommandName::DefaultGraph, "default-graph"),
        (CliCommandName::IuPin, "iu-pin"),
        (CliCommandName::Coverage, "coverage"),
        (CliCommandName::MaintenanceRange, "maintenance-range"),
        (CliCommandName::Mutants, "mutants"),
        (CliCommandName::Workflow, "workflow"),
        (CliCommandName::FuzzSmoke, "fuzz-smoke"),
    ];

    /// Every retained top-level command token parses and renders exactly.
    #[test]
    fn cli_command_tokens_cover_the_retained_inventory()
    {
        for &(command, spelling) in COMMAND_INVENTORY {
            assert!(matches!(CliCommandName::parse(spelling), Ok(parsed) if parsed == command));
            assert_eq!(command.as_str().as_ref(), spelling);
        }
        assert!(CliCommandName::parse("not-a-command").is_err());
    }

    #[cfg(feature = "fuzzing")]
    proptest! {
        /// Arbitrary byte streams never escape the parser-only facade.
        #[test]
        fn arbitrary_bytes_never_escape_the_facade(
            data in proptest::collection::vec(any::<u8>(), 0_usize..=MAX_FUZZ_INPUT_BYTES)
        ) {
            exercise_fuzz_input(&data);
        }
    }

    /// Invalid UTF-8 and oversized inputs return before parser dispatch.
    #[cfg(feature = "fuzzing")]
    #[test]
    fn invalid_utf8_and_oversized_inputs_skip_parser_dispatch()
    {
        let invalid = exercise_fuzz_bytes(&[0xff, 0xfe, 0xfd]);
        assert_eq!(invalid.input, InputDisposition::NonUtf8);
        assert_eq!(invalid.domains, ParserDomainReport::skipped());

        let mut oversized = Vec::from([b'a'; MAX_FUZZ_INPUT_BYTES]);
        oversized.push(b'a');
        let oversized = exercise_fuzz_bytes(&oversized);
        assert_eq!(oversized.input, InputDisposition::Oversized);
        assert_eq!(oversized.domains, ParserDomainReport::skipped());
    }

    /// Empty UTF-8 input still reaches parser domains that can accept
    /// emptiness.
    #[cfg(feature = "fuzzing")]
    #[test]
    fn empty_input_still_exercises_utf8_safe_domains()
    {
        let report = exercise_fuzz_bytes(&[]);
        assert_eq!(report.input, InputDisposition::Exercised);
        assert_eq!(report.domains.cli, ParserDomainStatus::Rejected);
        assert_eq!(report.domains.docs, ParserDomainStatus::Rejected);
        assert_eq!(report.domains.source_options, ParserDomainStatus::Completed);
        assert_eq!(
            report.domains.source_soundness,
            ParserDomainStatus::Completed
        );
    }

    /// Representative valid records reach every parser-only domain.
    #[cfg(feature = "fuzzing")]
    #[test]
    fn representative_valid_records_reach_each_parser_domain()
    {
        let cli = exercise_fuzz_bytes(b"coverage");
        assert_eq!(cli.domains.cli, ParserDomainStatus::Completed);

        let docs = exercise_fuzz_bytes(b"fmt");
        assert_eq!(docs.domains.docs, ParserDomainStatus::Completed);

        let options =
            exercise_fuzz_bytes(b"{-# OPTIONS --safe --without-K --hidden-argument-puns #-}\n");
        assert_eq!(
            options.domains.source_options,
            ParserDomainStatus::Completed
        );

        let soundness = exercise_fuzz_bytes(
            br#"#[test]
/// SOUNDNESS-ORACLE-COMPANION
fn coherence_companion() {}
"#,
        );
        assert_eq!(
            soundness.domains.source_soundness,
            ParserDomainStatus::Completed,
        );

        let coverage = exercise_fuzz_bytes(
            br#"target_percent = 80.00
[files]
"crates/demo/src/lib.rs" = 80.00
"#,
        );
        assert_eq!(coverage.domains.coverage, ParserDomainStatus::Completed);

        let mutation = exercise_fuzz_bytes(b"main HEAD");
        assert_eq!(mutation.domains.mutation, ParserDomainStatus::Completed);

        let maintenance = exercise_fuzz_bytes(
            br#"{"schema":1,"upper":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
        );
        assert_eq!(
            maintenance.domains.maintenance,
            ParserDomainStatus::Completed
        );
    }
}
