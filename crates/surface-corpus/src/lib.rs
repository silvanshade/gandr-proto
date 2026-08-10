//! The executable example corpus harness (ADR-52; epic `wyrd-61ck`, harness
//! bead `wyrd-on3m`).
//!
//! The corpus is **three trees** (`docs/workflow/corpus.md` —
//! never mixed):
//!
//! * [`MODEL_DIR`] holds the literate, learn-gandr-by-example programs;
//! * [`PATHOLOGICAL_DIR`] holds the semantic edge cases and failure goldens;
//! * [`SURFACE_DIR`] holds the **W4d syntax fold-in fixtures** (`wyrd-ku0f`):
//!   PBG-only parse-only programs exercising surface the committed tree-sitter
//!   grammar does not yet produce (`data` / `codata`, `def rec` + copatterns,
//!   `for` / `while` / `loop` / `break` / `continue`, `import`, string
//!   interpolation, and the reserved operation / rule / grade / GADT /
//!   attribute / fixity slots), plus the ruled circuit block form (`sign`
//!   blocks, the four-glyph arrow grid, two-sided ports, and the `node` /
//!   `feed` body statements) in the shapes that have **not** graduated — the
//!   reserved `<->` glyph, the `feed` wheel, the many-out node, an `oper`
//!   member's filler, and a `rule` member with no filler — plus the port
//!   discipline's own fixtures. The surface tree is **firewalled from
//!   execution**: this harness ([`check_case`], the corpus walker) runs the
//!   model and pathological trees only, so surface fixtures never lower or
//!   evaluate. Their gate is the PBG parser's zero-obligation corpus sweep
//!   (`gandr-parser` `acceptance::corpus_molds_to_zero_obligations`), which
//!   reads all three trees; the surface tree carries no `//@` directives. A
//!   surface fixture that is deliberately ill-formed **after** parsing — the
//!   shared-port refutations — lives here rather than in [`PATHOLOGICAL_DIR`],
//!   because the walker that reads that tree lowers what it finds and those
//!   fixtures' shapes have not graduated; its own gate is the named crate test
//!   that reads it.
//!
//! The ruled circuit **rule block** itself has graduated and is runnable: the
//! model witness is `examples/model/circuit/circuit-rule-block.gandr` and its
//! six declines — the many-out node, the wheel, the two-redex composite, the
//! cyclic wiring, the shared port, and the repeated port (the linearity
//! refusal reached from source) — are under
//! `examples/pathological/circuit/`.
//!
//! Each model / pathological example declares how to run itself and what to
//! expect through `//@` directives, and [`check_case`] runs one example
//! end-to-end — through
//! the REPL session engine ([`gandr_surface_engine::session::Session`],
//! preserving top-level item order for REPL-shaped expectations), the L-machine
//! runtime host ([`gandr_runtime_host::run_program`]), lowering alone, the
//! phase-L0 sequent inspector, or the stage-0 description elaborator —
//! returning the list of expectation failures (empty means pass).
//!
//! # Directives
//!
//! A directive is a line of the form `//@ key: value` (an ordinary line
//! comment to the grammar, so directives never perturb the program). Keys:
//!
//! | key | value | meaning |
//! |-----|-------|---------|
//! | `mode` | `session` / `shell` / `ffi` / `lower-only` / `sequent` / `desc` | how to run (default `session`) |
//! | `expect` | `clean` / `goal` / `lowers` | whole-run expectation |
//! | `expect-last-value` | rendered value | the last expression item returns this value |
//! | `expect-def` | name | a definition of `name` entered scope |
//! | `expect-diagnostic` | substring | some diagnostic message contains it |
//! | `expect-diagnostics-all` | substring | at least one diagnostic, and EVERY diagnostic message contains it |
//! | `expect-attribute` | schema name | a `Report.attributes` row has this schema (§5 projection) |
//! | `expect-stuck` | label | the last expression stuck with this label |
//! | `expect-blame` | label | the last expression blamed with this label |
//! | `expect-shell-value` | rendered value | the shell run returned this value |
//! | `expect-shell-exit` | integer | the shell run exited (`Proc::exit`) with this code |
//! | `expect-stdout-contains` | substring | the returned record's `stdout` field contains it |
//! | `expect-shell-error` | substring | preparing the shell run failed with this error |
//! | `expect-ffi-value` | rendered value | the ffi run returned this value |
//! | `expect-ffi-error` | substring | the ffi run aborted at the foreign boundary with this error |
//! | `expect-sequent-render` | rendered command | focusing produces this exact bounded command rendering |
//! | `expect-desc-render` | rendered description | stage-0 elaboration produces this exact description |
//! | `expect-desc-rules` | integer | elaborated descriptions carry this many rule faces in total |
//! | `expect-desc-store-cells` | integer | cell-layer elaboration puts this many cells in the stores |
//! | `expect-desc-cell-decline` | substring | some cell-layer decline message contains it |
//! | `expect-desc-decline` | substring | some stage-0 / declaration-table diagnostic contains it |
//! | `expect-desc-composites` | integer | the cell layer built this many whiskered composites |
//! | `expect-desc-unit-consumers` | `clean` | generic equality and serialization separate two unit constructors |
//! | `requires-feature` | `regex` / `ffi` | skip the example unless the named corpus feature is enabled |
//!
//! Expected values are written in [`render_value`]'s deliberately structural
//! grammar (booleans render as their `1 + 1` encoding `Inl(())` / `Inr(())`):
//! the surface pretty-printer is still owned by `wyrd-6n5m` / `wyrd-57er`, and
//! this harness must not grow a rival one — [`render_value`] is a test-side
//! notation, not a printer.
//!
//! # Why session mode keeps item boundaries
//!
//! [`gandr_surface_engine::session::Session::submit`] can process a whole
//! source submission, binding successful definitions before later items in that
//! same submission. The corpus harness still slices model examples into
//! top-level items so expectations can be checked against the same per-item
//! transcript a REPL user sees, while one accumulating session preserves
//! cross-item scope.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "the harness places its public drivers before the rendering helpers for readability, and the unit tests share fixture helpers called in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived pending a layout redesign"
    )
)]

use core::fmt::Write as _;

use gandr_core_checker::effect::host::FIELD_STDOUT;
use gandr_core_checker::outcome::Blame;
use gandr_core_checker::outcome::Eval;
use gandr_core_checker::outcome::StuckReason;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::NumLit;
use gandr_core_checker::syntax::Side;
use gandr_core_checker::syntax::Value;
use gandr_core_sequent::focus_term;
use gandr_core_sequent::pretty::render_command;
use gandr_core_sequent::wellformed;
use gandr_runtime_host::ShellOutcome;
use gandr_surface_engine::boundary::PipelineSource;
use gandr_surface_engine::desc_cells::DescCells;
use gandr_surface_engine::desc_cells::elaborate_desc_cells;
use gandr_surface_engine::desc_elab::ElabDiagnostic;
use gandr_surface_engine::desc_elab::elaborate_data_descs;
use gandr_surface_engine::lower::lower_source;
use gandr_surface_engine::lower::node_kinds;
use gandr_surface_engine::run::RunError as ShellRunError;
use gandr_surface_engine::run::run_source as run_shell_source;
use gandr_surface_engine::session::ItemOutcome;
use gandr_surface_engine::session::Session;
use gandr_surface_engine::synnode::SynTree;
use gandr_theory_levitation::Code;
use gandr_theory_levitation::DescValue;
use gandr_theory_levitation::Payload;
use gandr_theory_levitation::SignDesc;
use gandr_theory_levitation::generic_eq;
use gandr_theory_levitation::serialize_desc;
use gandr_theory_levitation::serialize_value;

/// The model-example tree, relative to the crate root: literate,
/// learn-gandr-by-example programs (ADR-52 Decision C).
pub const MODEL_DIR: &str = "examples/model";

/// The pathological tree, relative to the crate root: semantic edge cases and
/// failure goldens — testing artifacts, never pedagogy (ADR-52 Decision C).
pub const PATHOLOGICAL_DIR: &str = "examples/pathological";

/// The surface tree, relative to the crate root: the W4d syntax fold-in
/// fixtures (`wyrd-ku0f`).
///
/// PBG-only, parse-only programs (`data` / `codata`, `def rec`, control flow,
/// `import`, string interpolation, and the reserved slots) that the committed
/// tree-sitter grammar does not yet produce. This tree is **firewalled from
/// execution**: the corpus walker runs the model and pathological trees only,
/// so surface fixtures never lower or evaluate — their gate is the PBG parser's
/// zero-obligation sweep. Each opens with a literate graduation comment naming
/// the bead its semantics graduate under.
pub const SURFACE_DIR: &str = "examples/surface";

/// The maximum depth [`render_value`] descends before rendering `<deep>`
/// (bounded rendering; the ADR-47 posture applied to the harness).
const RENDER_DEPTH_LIMIT: RenderDepth = RenderDepth(32);

/// Current depth in the corpus harness's bounded value renderer.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RenderDepth(usize);

impl RenderDepth
{
    /// Root depth for a freshly rendered value.
    pub const ROOT: Self = Self(0);

    /// Descends one level without overflowing the host representation.
    #[inline]
    #[must_use]
    fn descend(self) -> Self
    {
        Self(self.0.saturating_add(1))
    }
}

impl From<usize> for RenderDepth
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

/// Stable corpus-harness vocabulary for outcomes, blame, and stuck states.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HarnessLabel(&'static str);

impl From<&'static str> for HarnessLabel
{
    #[inline]
    fn from(value: &'static str) -> Self
    {
        Self(value)
    }
}

impl AsRef<str> for HarnessLabel
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0
    }
}

impl core::fmt::Display for HarnessLabel
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        self.0.fmt(f)
    }
}

/// How an example asks to be run (the `mode:` directive).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode
{
    /// Submit the file's items, in order, to a fresh REPL session
    /// ([`gandr_surface_engine::session::Session::submit`]); the default.
    Session,
    /// Lower, link, and run the file under the L-machine runtime host
    /// ([`gandr_runtime_host::run_program`]); single-item programs and shell
    /// blocks reach the host handlers.
    Shell,
    /// Preserve and hash-check a native FFI example. Execution remains parked
    /// until the reboot FFI crate lands.
    Ffi,
    /// Only lower the file ([`gandr_surface_engine::lower::lower_source`]); for
    /// examples whose execution is deliberately not exercised in tests (e.g.
    /// network-touching walkthroughs).
    LowerOnly,
    /// Lower and focus the file into the phase-L0 sequent command IL, then
    /// inspect its bounded rendering and well-formedness.
    Sequent,
    /// Elaborate `data` / `codata` declarations into stage-0 descriptions, run
    /// their host-side generic consumers, and elaborate those descriptions on
    /// into the cell store
    /// ([`gandr_surface_engine::desc_cells::elaborate_desc_cells`]).
    Desc,
}

/// One expectation declared by an example (an `expect*` directive).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expect
{
    /// `expect: clean` — no diagnostics, no goals, and every item succeeded
    /// (definitions reported, expressions evaluated to terminal values).
    Clean,
    /// `expect: goal` — at least one hole goal was reported.
    Goal,
    /// `expect: lowers` — the source lowers (only meaningful with
    /// `mode: lower-only`).
    Lowers,
    /// `expect-last-value: v` — the last expression item evaluated to `ret w`
    /// whose [`render_value`] rendering equals `v`.
    LastValue(
        /// The expected rendering.
        String,
    ),
    /// `expect-def: name` — a definition of `name` typed and entered scope.
    Def(
        /// The defined name.
        String,
    ),
    /// `expect-diagnostic: s` — some diagnostic message contains `s`.
    Diagnostic(
        /// The required message substring.
        String,
    ),
    /// `expect-diagnostics-all: s` — the run produced at least one diagnostic
    /// AND every diagnostic message contains `s` (the ADR-76 K-rejection
    /// witness's "every diagnostic on the path" obligation, executable).
    DiagnosticsAll(
        /// The substring every diagnostic must contain.
        String,
    ),
    /// `expect-attribute: schema` — some `Report.attributes` row projects an
    /// attribute of this schema (proposal-attributes.md §5, the
    /// renderer-firewall projection).
    Attribute(
        /// The required schema name.
        String,
    ),
    /// `expect-stuck: label` — the last expression stuck with this label
    /// (see [`stuck_label`]).
    Stuck(
        /// The expected [`stuck_label`].
        String,
    ),
    /// `expect-blame: label` — the last expression blamed with this label
    /// (see [`blame_label`]).
    Blame(
        /// The expected [`blame_label`].
        String,
    ),
    /// `expect-shell-value: v` — the shell run completed with `ret w` whose
    /// [`render_value`] rendering equals `v`.
    ShellValue(
        /// The expected rendering.
        String,
    ),
    /// `expect-shell-exit: n` — the shell run exited via `Proc::exit n`.
    ShellExit(
        /// The expected exit code.
        i64,
    ),
    /// `expect-stdout-contains: s` — the shell run returned a record whose
    /// `stdout` field contains `s`.
    StdoutContains(
        /// The required stdout substring.
        String,
    ),
    /// `expect-shell-error: s` — lowering or linking the shell run failed with
    /// an error message containing `s`.
    ShellError(
        /// The required error substring.
        String,
    ),
    /// `expect-ffi-value: v` — the FFI run completed with `ret w` whose
    /// [`render_value`] rendering equals `v`.
    FfiValue(
        /// The expected rendering.
        String,
    ),
    /// `expect-ffi-error: s` — the parked FFI run is expected to abort at the
    /// foreign boundary once the reboot FFI runtime lands.
    FfiError(
        /// The required error substring.
        String,
    ),
    /// `expect-sequent-render: s` — some focused top-level item renders exactly
    /// as `s`.
    SequentRender(
        /// The expected bounded command rendering.
        String,
    ),
    /// `expect-desc-render: s` — some elaborated description renders exactly
    /// as `s`.
    DescRender(
        /// The expected inspectable description rendering.
        String,
    ),
    /// `expect-desc-rules: n` — the elaborated descriptions carry exactly `n`
    /// declared rule faces in total.
    DescRules(
        /// The expected total rule-face count.
        usize,
    ),
    /// `expect-desc-store-cells: n` — elaborating the descriptions into the
    /// cell layer puts exactly `n` cells in the stores (a frame-defining cell
    /// per declared constructor plus every admitted rule cell).
    DescStoreCells(
        /// The expected total stored-cell count.
        usize,
    ),
    /// `expect-desc-decline: s` — some stage-0 elaboration or declaration-table
    /// diagnostic contains `s`.
    ///
    /// Its presence also *licenses* the diagnostics: a `desc`-mode example
    /// carrying one is asserting a decline, so the harness stops treating any
    /// diagnostic as an outright failure and checks the substrings instead.
    DescDecline(
        /// The substring some diagnostic must contain.
        String,
    ),
    /// `expect-desc-composites: n` — the cell layer built exactly `n` whiskered
    /// composites, one per admitted circuit rule member.
    DescComposites(
        /// The expected composite count.
        usize,
    ),
    /// `expect-desc-cell-decline: s` — some cell-layer decline message contains
    /// `s` (the honest-limits half of the description → cells wire).
    DescCellDecline(
        /// The required decline-message substring.
        String,
    ),
    /// `expect-desc-unit-consumers: clean` — the first description's first two
    /// constructors are nullary and the generic equality/serialization
    /// consumers agree on equal values and distinguish the constructors.
    DescUnitConsumers,
}

/// A corpus crate feature an example requires before it can run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequiredFeature
{
    /// The source-facing `regex.extract` builtin, enabled by the `regex`
    /// feature on this crate.
    Regex,
    /// Native FFI execution for `mode: ffi` examples, enabled by the `ffi`
    /// feature on this crate.
    Ffi,
}

/// A parsed example: its run mode, feature requirements, and declared
/// expectations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Case
{
    /// How to run the example.
    pub mode: Mode,
    /// Corpus crate features that must be enabled before this example runs.
    pub required_features: Vec<RequiredFeature>,
    /// The declared expectations, in directive order (at least one).
    pub expects: Vec<Expect>,
}

/// Runs one example source end-to-end and evaluates its expectations.
///
/// # Contract
/// - ensures: returns one human-readable failure per unmet expectation (or per
///   infrastructure failure); an empty vector means the example passes.
/// - provides: the corpus walker tests' single entry point, and the seam the
///   instrumentation surface (`wyrd-9onc`) extends.
#[inline]
#[must_use]
pub fn check_case<'source, T>(source: T) -> Vec<String>
where
    T: Into<PipelineSource<'source>>,
{
    let source = source.into();
    let case = match parse_case(source) {
        | Ok(parsed) => parsed,
        | Err(error) => return vec![format!("directive error: {error}")],
    };
    if case.required_features.iter().any(|&feature| match feature {
        | RequiredFeature::Regex => !cfg!(feature = "regex"),
        | RequiredFeature::Ffi => !cfg!(feature = "ffi"),
    }) {
        return Vec::new();
    }
    match case.mode {
        | Mode::Session => check_session(source, &case.expects),
        | Mode::Shell => check_shell(source, &case.expects),
        | Mode::Ffi => check_ffi(source, &case.expects),
        | Mode::LowerOnly => check_lower_only(source, &case.expects),
        | Mode::Sequent => check_sequent(source, &case.expects),
        | Mode::Desc => check_desc(source, &case.expects),
    }
}

/// Parses the `//@` directives of one example source.
///
/// - ensures: returns the declared [`Mode`] (default [`Mode::Session`]),
///   feature requirements, and expectations in directive order; every example
///   must declare at least one expectation (an assertion-free example silently
///   asserts nothing — rejected).
///
/// # Errors
/// A human-readable description of the first malformed, unknown, or missing
/// directive.
#[inline]
pub fn parse_case<'source, T>(source: T) -> Result<Case, String>
where
    T: Into<PipelineSource<'source>>,
{
    let source = source.into();
    let mut mode: Option<Mode> = None;
    let mut required_features: Vec<RequiredFeature> = Vec::new();
    let mut expects: Vec<Expect> = Vec::new();
    for line in source.lines() {
        let Some(directive) = line.trim().strip_prefix("//@")
        else {
            continue;
        };
        let (key, value) = directive
            .split_once(':')
            .ok_or_else(|| format!("malformed directive (no `:`): `{directive}`"))?;
        let key = key.trim();
        let value = value.trim();
        match key {
            | "mode" => {
                let parsed = match value {
                    | "session" => Mode::Session,
                    | "shell" => Mode::Shell,
                    | "ffi" => Mode::Ffi,
                    | "lower-only" => Mode::LowerOnly,
                    | "sequent" => Mode::Sequent,
                    | "desc" => Mode::Desc,
                    | other => return Err(format!("unknown mode `{other}`")),
                };
                if mode.replace(parsed).is_some() {
                    return Err("duplicate `mode` directive".to_owned());
                }
            },
            | "requires-feature" => {
                let feature = match value {
                    | "regex" => RequiredFeature::Regex,
                    | "ffi" => RequiredFeature::Ffi,
                    | other => return Err(format!("unknown required feature `{other}`")),
                };
                if required_features.contains(&feature) {
                    return Err(format!("duplicate required feature `{value}`"));
                }
                required_features.push(feature);
            },
            | "expect" => match value {
                | "clean" => expects.push(Expect::Clean),
                | "goal" => expects.push(Expect::Goal),
                | "lowers" => expects.push(Expect::Lowers),
                | other => return Err(format!("unknown expectation `{other}`")),
            },
            | "expect-last-value" => expects.push(Expect::LastValue(value.to_owned())),
            | "expect-def" => expects.push(Expect::Def(value.to_owned())),
            | "expect-diagnostic" => expects.push(Expect::Diagnostic(value.to_owned())),
            | "expect-diagnostics-all" => expects.push(Expect::DiagnosticsAll(value.to_owned())),
            | "expect-attribute" => expects.push(Expect::Attribute(value.to_owned())),
            | "expect-stuck" => expects.push(Expect::Stuck(value.to_owned())),
            | "expect-blame" => expects.push(Expect::Blame(value.to_owned())),
            | "expect-shell-value" => expects.push(Expect::ShellValue(value.to_owned())),
            | "expect-shell-exit" => {
                let code: i64 = value
                    .parse()
                    .map_err(|_ignored| format!("non-integer exit code `{value}`"))?;
                expects.push(Expect::ShellExit(code));
            },
            | "expect-stdout-contains" => expects.push(Expect::StdoutContains(value.to_owned())),
            | "expect-shell-error" => expects.push(Expect::ShellError(value.to_owned())),
            | "expect-ffi-value" => expects.push(Expect::FfiValue(value.to_owned())),
            | "expect-ffi-error" => expects.push(Expect::FfiError(value.to_owned())),
            | "expect-sequent-render" => expects.push(Expect::SequentRender(value.to_owned())),
            | "expect-desc-render" => expects.push(Expect::DescRender(value.to_owned())),
            | "expect-desc-rules" => {
                let count: usize = value
                    .parse()
                    .map_err(|_ignored| format!("non-integer description cell count `{value}`"))?;
                expects.push(Expect::DescRules(count));
            },
            | "expect-desc-store-cells" => {
                let count: usize = value
                    .parse()
                    .map_err(|_ignored| format!("non-integer stored cell count `{value}`"))?;
                expects.push(Expect::DescStoreCells(count));
            },
            | "expect-desc-cell-decline" => expects.push(Expect::DescCellDecline(value.to_owned())),
            | "expect-desc-decline" => expects.push(Expect::DescDecline(value.to_owned())),
            | "expect-desc-composites" => {
                expects.push(Expect::DescComposites(
                    value.parse::<usize>().map_err(|error| error.to_string())?,
                ));
            },
            | "expect-desc-unit-consumers" => {
                if value != "clean" {
                    return Err(format!(
                        "unknown description-consumer expectation `{value}`"
                    ));
                }
                expects.push(Expect::DescUnitConsumers);
            },
            | other => return Err(format!("unknown directive key `{other}`")),
        }
    }
    if expects.is_empty() {
        return Err("an example must declare at least one `expect*` directive".to_owned());
    }
    Ok(Case {
        mode: mode.unwrap_or(Mode::Session),
        required_features,
        expects,
    })
}

/// The aggregated result of one session-mode run: every diagnostic message,
/// goal, and item outcome, across the file's ordered item submissions.
struct SessionRun
{
    /// Every diagnostic message, in submission order.
    diagnostics: Vec<String>,
    /// The total number of hole goals reported.
    goals: usize,
    /// Every item outcome, in submission order.
    outcomes: Vec<ItemOutcome>,
    /// The schema name of every projected `Report.attributes` row, in
    /// submission order (the entity-attribute projection, §5).
    attributes: Vec<String>,
}

/// Runs a session-mode example: the file's items are submitted in order to a
/// single accumulating session, and the expectations are evaluated against the
/// aggregated [`SessionRun`].
fn check_session(
    source: PipelineSource<'_>,
    expects: &[Expect],
) -> Vec<String>
{
    let items = match split_items(source) {
        | Ok(items) => items,
        | Err(error) => return vec![error],
    };
    if items.is_empty() {
        return vec!["the example contains no top-level items".to_owned()];
    }
    let mut session = Session::new();
    let mut run = SessionRun {
        diagnostics: Vec::new(),
        goals: 0,
        outcomes: Vec::new(),
        attributes: Vec::new(),
    };
    for item in &items {
        match session.submit(item) {
            | Ok(submission) => {
                for diagnostic in &submission.report.diagnostics {
                    run.diagnostics.push(diagnostic.message.clone());
                }
                run.goals = run.goals.saturating_add(submission.report.goals.len());
                for attribute in &submission.report.attributes {
                    run.attributes.push(attribute.schema.clone());
                }
                run.outcomes.extend(submission.outcomes.iter().cloned());
            },
            | Err(error) => return vec![format!("infrastructure lowering failed: {error}")],
        }
    }
    expects
        .iter()
        .filter_map(|expect| session_failure(&run, expect))
        .collect()
}

/// Slices `source` into its top-level items (comments and the shebang are
/// grammar extras, skipped), preserving source order.
///
/// # Contract
/// - ensures: returns each top-level non-comment item's source text, in order;
///   the concatenation of items covers exactly the program content.
///
/// # Errors
/// The parse front-end failing to assemble the tree (an arena-construction
/// failure at commit; never ungrammatical input — the melder parse is total).
#[inline]
pub fn split_items<'source, T>(source: T) -> Result<Vec<String>, String>
where
    T: Into<PipelineSource<'source>>,
{
    let source = source.into();
    // The melder push-machine front-end
    // ([`gandr_surface_engine::synnode::SynTree`], the tree-sitter-free parse
    // core) replaces the retired tree-sitter parser: its `source_file` view
    // yields the same ordered top-level items (trivia space-skipped, grout
    // unwrapped) the lowerer walks.
    let tree =
        SynTree::parse(source.as_ref()).map_err(|error| format!("grammar unavailable: {error}"))?;
    let root = tree.root();
    let mut spans: Vec<(bool, core::ops::Range<usize>)> = Vec::new();
    for node in root.named_children() {
        // The `source_file` view already space-skips comments/shebangs; the
        // `EXTRAS` filter mirrors the lowerer's defensive guard so a future
        // trivia-as-named-child change cannot leak a comment into an item span.
        if node_kinds::EXTRAS.contains(&node.kind().as_ref()) {
            continue;
        }
        spans.push((
            node.kind() == node_kinds::DEF_SIGNATURE,
            node.byte_range().into(),
        ));
    }
    let mut items = Vec::new();
    let mut iter = spans.into_iter().peekable();
    while let Some((is_signature, range)) = iter.next() {
        // A `def name : T;` signature only carries its ascription into the
        // paired `def name(...)` definition when both lower together
        // (the lowered item's `ascription`), so keep the pair in one slice.
        let mut end = range.end;
        if is_signature {
            let paired_end = iter.peek().map(
                |&(
                    _,
                    core::ops::Range {
                        end: ref next_end, ..
                    },
                )| *next_end,
            );
            if let Some(next_end) = paired_end {
                end = next_end;
                let _paired = iter.next();
            }
        }
        let slice = source
            .get(range.start .. end)
            .ok_or_else(|| "item span out of bounds".to_owned())?;
        items.push(slice.to_owned());
    }
    Ok(items)
}

/// Evaluates one expectation against a session run, returning the failure
/// message when unmet.
fn session_failure(
    run: &SessionRun,
    expect: &Expect,
) -> Option<String>
{
    match *expect {
        | Expect::Clean => clean_failure(run),
        | Expect::Goal => {
            (run.goals == 0).then(|| "expected at least one goal; none reported".to_owned())
        },
        | Expect::LastValue(ref expected) => {
            let Some(eval) = last_expression(run)
            else {
                return Some("expected a last expression item; none found".to_owned());
            };
            match returned_value(eval) {
                | Some(value) => {
                    let rendered = render_value(value, 0);
                    if rendered == *expected {
                        None
                    }
                    else {
                        Some(format!(
                            "last value mismatch: expected `{expected}`, got `{rendered}`"
                        ))
                    }
                },
                | None => Some(format!(
                    "expected the last expression to return `{expected}`, but it did not \
                     terminate with `ret v` (label: `{}`)",
                    eval_label(eval)
                )),
            }
        },
        | Expect::Def(ref name) => {
            let bound = run.outcomes.iter().any(|outcome| {
                matches!(
                    *outcome,
                    ItemOutcome::Definition { name: ref defined, bound: true, .. }
                        if defined == name
                )
            });
            if bound {
                None
            }
            else {
                Some(format!("expected a bound definition of `{name}`"))
            }
        },
        | Expect::Diagnostic(ref needle) => {
            let found = run
                .diagnostics
                .iter()
                .any(|message| message.contains(needle));
            if found {
                None
            }
            else {
                Some(format!(
                    "expected a diagnostic containing `{needle}`; got: {}",
                    diagnostic_summary(run)
                ))
            }
        },
        | Expect::DiagnosticsAll(ref needle) => {
            if run.diagnostics.is_empty() {
                return Some(format!(
                    "expected at least one diagnostic (each containing `{needle}`); got none"
                ));
            }
            let all = run
                .diagnostics
                .iter()
                .all(|message| message.contains(needle));
            if all {
                None
            }
            else {
                Some(format!(
                    "expected EVERY diagnostic to contain `{needle}`; got: {}",
                    diagnostic_summary(run)
                ))
            }
        },
        | Expect::Stuck(ref label) => match last_expression(run) {
            | Some(&Eval::Stuck(ref reason)) if stuck_label(reason).as_ref() == label.as_str() => {
                None
            },
            | Some(eval) => Some(format!(
                "expected stuck `{label}`; got `{}`",
                eval_label(eval)
            )),
            | None => Some("expected a last expression item; none found".to_owned()),
        },
        | Expect::Blame(ref label) => match last_expression(run) {
            | Some(&Eval::Blame(ref blame)) if blame_label(blame).as_ref() == label.as_str() => {
                None
            },
            | Some(eval) => Some(format!(
                "expected blame `{label}`; got `{}`",
                eval_label(eval)
            )),
            | None => Some("expected a last expression item; none found".to_owned()),
        },
        | Expect::Attribute(ref schema) => {
            if run.attributes.iter().any(|projected| projected == schema) {
                None
            }
            else {
                Some(format!(
                    "expected a projected attribute of schema `{schema}`; projected: {}",
                    attribute_summary(run)
                ))
            }
        },
        | Expect::Lowers
        | Expect::ShellValue(_)
        | Expect::ShellExit(_)
        | Expect::StdoutContains(_)
        | Expect::ShellError(_)
        | Expect::FfiValue(_)
        | Expect::FfiError(_)
        | Expect::SequentRender(_)
        | Expect::DescRender(_)
        | Expect::DescRules(_)
        | Expect::DescStoreCells(_)
        | Expect::DescCellDecline(_)
        | Expect::DescDecline(_)
        | Expect::DescComposites(_)
        | Expect::DescUnitConsumers => Some("directive is not valid in session mode".to_owned()),
    }
}

/// The `expect: clean` predicate: no diagnostics, no goals, every definition
/// reported and every expression terminated with a value.
fn clean_failure(run: &SessionRun) -> Option<String>
{
    if !run.diagnostics.is_empty() {
        return Some(format!(
            "expected a clean run; diagnostics: {}",
            diagnostic_summary(run)
        ));
    }
    if run.goals != 0 {
        return Some(format!(
            "expected a clean run; {} goal(s) reported",
            run.goals
        ));
    }
    for (index, outcome) in run.outcomes.iter().enumerate() {
        let failure = match *outcome {
            | ItemOutcome::Definition { .. } => None,
            | ItemOutcome::Expression { ref value, .. } => match *value {
                | Eval::Value(_) => None,
                | ref other => Some(format!(
                    "item {index}: expression did not terminate with a value (`{}`)",
                    eval_label(other)
                )),
            },
            | ref other => Some(format!(
                "item {index}: non-success outcome `{}`",
                outcome_label(other)
            )),
        };
        if failure.is_some() {
            return failure;
        }
    }
    None
}

/// The last expression item's evaluation outcome, if any.
fn last_expression(run: &SessionRun) -> Option<&Eval>
{
    run.outcomes.iter().rev().find_map(|outcome| {
        if let ItemOutcome::Expression { ref value, .. } = *outcome {
            Some(value)
        }
        else {
            None
        }
    })
}

/// The returned value of a terminal `ret v` evaluation, if that is what
/// `eval` is.
fn returned_value(eval: &Eval) -> Option<&Value>
{
    match *eval {
        | Eval::Value(Comp::Ret(ref value)) => Some(value.as_ref()),
        | _ => None,
    }
}

/// A one-line summary of a run's diagnostics (for failure messages).
fn diagnostic_summary(run: &SessionRun) -> String
{
    if run.diagnostics.is_empty() {
        return "(none)".to_owned();
    }
    let messages: Vec<&str> = run.diagnostics.iter().map(String::as_str).collect();
    messages.join(" | ")
}

/// A stable label for an [`Eval`] (failure messages and directive matching).
fn eval_label(eval: &Eval) -> String
{
    match *eval {
        | Eval::Value(Comp::Ret(ref value)) => format!("ret {}", render_value(value, 0)),
        | Eval::Value(_) => "non-ret terminal".to_owned(),
        | Eval::Stuck(ref reason) => format!("stuck:{}", stuck_label(reason)),
        | Eval::Blame(ref blame) => format!("blame:{}", blame_label(blame)),
    }
}

/// A stable label for a [`StuckReason`] (the `expect-stuck` directive's
/// vocabulary).
#[inline]
#[must_use]
pub fn stuck_label(reason: &StuckReason) -> HarnessLabel
{
    match *reason {
        | StuckReason::AppliedNonFunction => "applied-non-function".into(),
        | StuckReason::SequencedNonReturner => "sequenced-non-returner".into(),
        | StuckReason::ForcedNonThunk => "forced-non-thunk".into(),
        | StuckReason::CasedNonSum => "cased-non-sum".into(),
        | StuckReason::ListCasedNonList => "list-cased-non-list".into(),
        | StuckReason::SplitNonProduct => "split-non-product".into(),
        | StuckReason::ProjectedNonPair => "projected-non-pair".into(),
        | StuckReason::RecordProjNonRecord => "record-proj-non-record".into(),
        | StuckReason::RecordProjMissingField => "record-proj-missing-field".into(),
        | StuckReason::ResumedNonStack => "resumed-non-stack".into(),
        | StuckReason::UnsupportedByReference => "unsupported-by-reference".into(),
        | StuckReason::InvalidClosureBody => "invalid-closure-body".into(),
        | StuckReason::StepLimit => "step-limit".into(),
        | _ => "stuck".into(),
    }
}

/// A stable label for a [`Blame`] (the `expect-blame` directive's vocabulary).
#[inline]
#[must_use]
pub fn blame_label(blame: &Blame) -> HarnessLabel
{
    match *blame {
        | Blame::Hole => "hole".into(),
        | Blame::ShiftNoReset => "shift-no-reset".into(),
        | Blame::PerformNoHandler => "perform-no-handler".into(),
    }
}

/// A stable label for an [`ItemOutcome`] variant (failure messages only).
fn outcome_label(outcome: &ItemOutcome) -> HarnessLabel
{
    match *outcome {
        | ItemOutcome::Definition { .. } => "definition".into(),
        | ItemOutcome::Expression { .. } => "expression".into(),
        | ItemOutcome::TypeError { .. } => "type-error".into(),
        | ItemOutcome::Holey => "holey".into(),
        | ItemOutcome::Unknown => "unknown".into(),
    }
}

/// A one-line summary of a run's projected attribute schemas (for failure
/// messages).
fn attribute_summary(run: &SessionRun) -> String
{
    if run.attributes.is_empty() {
        return "(none)".to_owned();
    }
    run.attributes.join(", ")
}

/// Runs a shell-mode example through the surface engine and L-machine host.
fn check_shell(
    source: PipelineSource<'_>,
    expects: &[Expect],
) -> Vec<String>
{
    let run = run_shell_source(source);
    expects
        .iter()
        .filter_map(|expect| shell_failure(&run, expect))
        .collect()
}

/// Evaluates one expectation against a shell run outcome.
fn shell_failure(
    run: &Result<ShellOutcome, ShellRunError>,
    expect: &Expect,
) -> Option<String>
{
    match *expect {
        | Expect::ShellError(ref needle) => match *run {
            | Err(ref error) => {
                let message = error.to_string();
                if message.contains(needle) {
                    None
                }
                else {
                    Some(format!(
                        "expected a shell error containing `{needle}`; got `{message}`"
                    ))
                }
            },
            | Ok(_) => Some(format!(
                "expected a shell error containing `{needle}`; the run was prepared"
            )),
        },
        | Expect::Clean => match *run {
            | Ok(ref outcome) if outcome.returned().is_some() => None,
            | Ok(_) => Some("expected the shell run to return a value".to_owned()),
            | Err(ref error) => Some(format!("shell run failed to prepare: {error}")),
        },
        | Expect::ShellValue(ref expected) => match *run {
            | Ok(ref outcome) => match outcome.returned() {
                | Some(value) => {
                    let rendered = render_value(value, 0);
                    if rendered == *expected {
                        None
                    }
                    else {
                        Some(format!(
                            "shell value mismatch: expected `{expected}`, got `{rendered}`"
                        ))
                    }
                },
                | None => Some(format!(
                    "expected the shell run to return `{expected}`; it did not return a value"
                )),
            },
            | Err(ref error) => Some(format!("shell run failed to prepare: {error}")),
        },
        | Expect::ShellExit(expected) => match *run {
            | Ok(ShellOutcome::Exited { code }) if code == expected => None,
            | Ok(ref other) => Some(format!(
                "expected `Proc::exit {expected}`; the run completed differently ({})",
                shell_label(other)
            )),
            | Err(ref error) => Some(format!("shell run failed to prepare: {error}")),
        },
        | Expect::StdoutContains(ref needle) => match *run {
            | Ok(ref outcome) => {
                let stdout = outcome
                    .returned()
                    .and_then(Value::as_record)
                    .and_then(|fields| fields.get(FIELD_STDOUT))
                    .and_then(|value| value.as_str());
                match stdout {
                    | Some(text) if text.as_ref().contains(needle) => None,
                    | Some(text) => Some(format!(
                        "expected stdout to contain `{needle}`; got `{}`",
                        text.as_ref()
                    )),
                    | None => Some("the shell run returned no `stdout` field".to_owned()),
                }
            },
            | Err(ref error) => Some(format!("shell run failed to prepare: {error}")),
        },
        | _ => Some("directive is not valid in shell mode".to_owned()),
    }
}

/// Keeps FFI-mode sources in the frozen corpus until the reboot FFI crate
/// lands. Their bytes and directive shape remain covered by the corpus gates;
/// execution is deliberately unavailable in this crate.
fn check_ffi(
    _source: PipelineSource<'_>,
    _expects: &[Expect],
) -> Vec<String>
{
    Vec::new()
}

/// Renders a machine [`Value`] into the harness's structural notation.
///
/// This is a **test-side notation**, not a pretty-printer (that surface is
/// owned by `wyrd-6n5m` / `wyrd-57er`): annotations are transparent, booleans
/// appear as their `1 + 1` encoding (`Inl(())` / `Inr(())`), thunks render
/// opaquely, and anything unrecognized renders `<opaque>`. Rendering is
/// depth-bounded (`<deep>` beyond [`RENDER_DEPTH_LIMIT`]).
///
/// # Contract
/// - ensures: deterministic output for a given value (records iterate their
///   `BTreeMap` order); total — never panics.
#[inline]
#[must_use]
pub fn render_value<T>(
    value: &Value,
    depth: T,
) -> String
where
    T: Into<RenderDepth>,
{
    enum RenderStep<'value>
    {
        Value
        {
            value: &'value Value,
            depth: RenderDepth,
        },
        Text(&'value str),
    }

    let mut output = String::new();
    let mut steps = vec![RenderStep::Value {
        value,
        depth: depth.into(),
    }];
    while let Some(step) = steps.pop() {
        let RenderStep::Value { value, depth } = step
        else {
            let RenderStep::Text(text) = step
            else {
                continue;
            };
            output.push_str(text);
            continue;
        };
        if depth >= RENDER_DEPTH_LIMIT {
            output.push_str("<deep>");
            continue;
        }
        let below = depth.descend();
        match *value {
            | Value::Unit => output.push_str("()"),
            | Value::Int(int) => {
                let _infallible = write!(&mut output, "{int}");
            },
            | Value::Str(ref text) => {
                output.push('"');
                output.push_str(text);
                output.push('"');
            },
            | Value::Num(num) => output.push_str(&render_num(num)),
            | Value::Pair(ref fst, ref snd) => {
                steps.push(RenderStep::Text(")"));
                steps.push(RenderStep::Value {
                    value: snd.as_ref(),
                    depth: below,
                });
                steps.push(RenderStep::Text(", "));
                steps.push(RenderStep::Value {
                    value: fst.as_ref(),
                    depth: below,
                });
                steps.push(RenderStep::Text("("));
            },
            | Value::Inj(side, ref payload) => {
                let prefix = match side {
                    | Side::Fst => "Inl(",
                    | Side::Snd => "Inr(",
                };
                steps.push(RenderStep::Text(")"));
                steps.push(RenderStep::Value {
                    value: payload.as_ref(),
                    depth: below,
                });
                steps.push(RenderStep::Text(prefix));
            },
            | Value::List(ref items) => {
                steps.push(RenderStep::Text("]"));
                for (index, item) in items.iter().enumerate().rev() {
                    if index.saturating_add(1) < items.len() {
                        steps.push(RenderStep::Text(", "));
                    }
                    steps.push(RenderStep::Value {
                        value: item.as_ref(),
                        depth: below,
                    });
                }
                steps.push(RenderStep::Text("["));
            },
            | Value::Record(ref fields) => {
                steps.push(RenderStep::Text("}"));
                for (index, (label, field)) in fields.iter().enumerate().rev() {
                    if index.saturating_add(1) < fields.len() {
                        steps.push(RenderStep::Text(", "));
                    }
                    steps.push(RenderStep::Value {
                        value: field.as_ref(),
                        depth: below,
                    });
                    steps.push(RenderStep::Text(" = "));
                    steps.push(RenderStep::Text(label));
                }
                steps.push(RenderStep::Text("#{"));
            },
            | Value::Thunk(..) => output.push_str("<thunk>"),
            | Value::Annot(ref payload, _) => steps.push(RenderStep::Value {
                value: payload.as_ref(),
                depth: below,
            }),
            | Value::Var(ref name) => {
                output.push_str("<var ");
                output.push_str(name.as_ref());
                output.push('>');
            },
            // A reflexivity proof renders through its witness (ADR-76): the
            // canonical inhabitant of a closed identity type, `here(4)`.
            | Value::Here(ref witness) => {
                steps.push(RenderStep::Text(")"));
                steps.push(RenderStep::Value {
                    value: witness.as_ref(),
                    depth: below,
                });
                steps.push(RenderStep::Text("here("));
            },
            | _ => output.push_str("<opaque>"),
        }
    }
    output
}

/// Renders a typed numeric literal (`5u32`, `1.5f64`, …).
fn render_num(num: NumLit) -> String
{
    match num {
        | NumLit::U32(n) => format!("{n}u32"),
        | NumLit::U64(n) => format!("{n}u64"),
        | NumLit::I32(n) => format!("{n}i32"),
        | NumLit::I64(n) => format!("{n}i64"),
        | NumLit::F32(bits) => format!("{}f32", f32::from_bits(bits)),
        | NumLit::F64(bits) => format!("{}f64", f64::from_bits(bits)),
    }
}

/// A one-line label for a [`ShellOutcome`] (failure messages only).
fn shell_label(outcome: &ShellOutcome) -> String
{
    match *outcome {
        | ShellOutcome::Completed(_) => "completed".to_owned(),
        | ShellOutcome::Exited { code } => format!("exited with {code}"),
        | ShellOutcome::HostFailed(ref error) => format!("host failed: {error}"),
    }
}

/// Runs a lower-only example: the source must lower; nothing is executed.
fn check_lower_only(
    source: PipelineSource<'_>,
    expects: &[Expect],
) -> Vec<String>
{
    let lower_result = lower_source(source);
    let mut failures = Vec::new();
    for expect in expects {
        match *expect {
            | Expect::Lowers => {
                if let Err(ref error) = lower_result {
                    failures.push(format!("expected the source to lower; got: {error}"));
                }
            },
            | _ => failures.push("directive is not valid in lower-only mode".to_owned()),
        }
    }
    failures
}

/// Runs a sequent-inspection example through the phase-L0 focusing translation.
fn check_sequent(
    source: PipelineSource<'_>,
    expects: &[Expect],
) -> Vec<String>
{
    let lowered = match lower_source(source) {
        | Ok(lowered) => lowered,
        | Err(error) => return vec![format!("sequent example failed to lower: {error}")],
    };
    if lowered.items.is_empty() {
        return vec!["the sequent example contains no top-level items".to_owned()];
    }
    let mut rendered = Vec::new();
    for (index, item) in lowered.items.iter().enumerate() {
        let focused = match focus_term(&item.term) {
            | Ok(focused) => focused,
            | Err(error) => {
                return vec![format!("sequent item {index} failed to focus: {error}")];
            },
        };
        match wellformed(focused.arena(), focused.root()) {
            | Ok(frees) if frees.covars.is_empty() => {},
            | Ok(frees) => {
                return vec![format!(
                    "sequent item {index} has free covariables: {:?}",
                    frees.covars
                )];
            },
            | Err(error) => {
                return vec![format!(
                    "sequent item {index} failed the typed-IL check: {error}"
                )];
            },
        }
        rendered.push(render_command(focused.arena(), focused.root()));
    }
    expects
        .iter()
        .filter_map(|expect| match *expect {
            | Expect::SequentRender(ref expected)
                if rendered.iter().any(|actual| actual == expected) =>
            {
                None
            },
            | Expect::SequentRender(ref expected) => Some(format!(
                "expected a focused command `{expected}`; got {}",
                rendered.join(" | ")
            )),
            | _ => Some("directive is not valid in sequent mode".to_owned()),
        })
        .collect()
}

/// Runs a stage-0 description example through elaboration and generic
/// consumers.
fn check_desc(
    source: PipelineSource<'_>,
    expects: &[Expect],
) -> Vec<String>
{
    let elaborated = elaborate_data_descs(source);
    // An example asserting a decline has said so with `expect-desc-decline`;
    // without one, any diagnostic is an outright failure, which is what keeps a
    // model example honest about elaborating cleanly.
    let asserts_decline = expects
        .iter()
        .any(|expect| matches!(*expect, Expect::DescDecline(_)));
    if !elaborated.diagnostics.is_empty() && !asserts_decline {
        let messages: Vec<&str> = elaborated
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        return vec![format!(
            "description elaboration reported diagnostics: {}",
            messages.join(" | ")
        )];
    }
    if elaborated.descs.is_empty() && !asserts_decline {
        return vec!["the description example elaborated no declarations".to_owned()];
    }
    let rendered: Vec<String> = elaborated
        .descs
        .iter()
        .map(|desc| String::from(serialize_desc(desc)))
        .collect();
    // The second half of the stage-0 path: the descriptions elaborated into the
    // content-addressed cell store, with the cell layer's own declines.
    let cells = elaborate_desc_cells(&elaborated.descs);
    expects
        .iter()
        .filter_map(|expect| match *expect {
            | Expect::DescRender(ref expected)
                if rendered.iter().any(|actual| actual == expected) =>
            {
                None
            },
            | Expect::DescRender(ref expected) => Some(format!(
                "expected description `{expected}`; got {}",
                rendered.join(" | ")
            )),
            | Expect::DescRules(expected) => {
                let actual = elaborated.descs.iter().fold(0_usize, |total, desc| {
                    total.saturating_add(desc.rules.len())
                });
                if actual == expected {
                    None
                }
                else {
                    Some(format!(
                        "expected {expected} description rule face(s); got {actual}"
                    ))
                }
            },
            | Expect::DescStoreCells(expected) => {
                let actual = cells.stores.iter().fold(0_usize, |total, store| {
                    total.saturating_add(usize::from(store.len()))
                });
                if actual == expected {
                    None
                }
                else {
                    Some(format!(
                        "expected {expected} elaborated cell(s) in the store(s); got {actual}"
                    ))
                }
            },
            | Expect::DescCellDecline(ref needle) => {
                let found = cells
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(needle));
                if found {
                    None
                }
                else {
                    Some(format!(
                        "expected a cell-layer decline containing `{needle}`; got {}",
                        cell_decline_summary(&cells)
                    ))
                }
            },
            | Expect::DescDecline(ref needle) => {
                let found = elaborated
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(needle));
                if found {
                    None
                }
                else {
                    Some(format!(
                        "expected an elaboration decline containing `{needle}`; got {}",
                        elab_decline_summary(&elaborated.diagnostics)
                    ))
                }
            },
            | Expect::DescComposites(expected) => {
                let actual = cells.composites.len();
                if actual == expected {
                    None
                }
                else {
                    Some(format!(
                        "expected {expected} whiskered composite(s); got {actual}"
                    ))
                }
            },
            | Expect::DescUnitConsumers => desc_unit_consumer_failure(&elaborated.descs),
            | _ => Some("directive is not valid in desc mode".to_owned()),
        })
        .collect()
}

/// A one-line summary of a stage-0 elaboration's diagnostics (for failure
/// messages).
fn elab_decline_summary(diagnostics: &[ElabDiagnostic]) -> String
{
    if diagnostics.is_empty() {
        return "(none)".to_owned();
    }
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<&str>>()
        .join(" | ")
}

/// A one-line summary of a cell-layer elaboration's declines (for failure
/// messages).
fn cell_decline_summary(cells: &DescCells) -> String
{
    if cells.diagnostics.is_empty() {
        return "(none)".to_owned();
    }
    let messages: Vec<&str> = cells
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();
    messages.join(" | ")
}

/// Checks the stage-0 generic consumers on two nullary constructors.
fn desc_unit_consumer_failure(descs: &[SignDesc]) -> Option<String>
{
    let Some(desc) = descs.first()
    else {
        return Some("description consumer check has no declaration".to_owned());
    };
    let Some(first_ctor) = desc.ctors.first()
    else {
        return Some("description consumer check needs a first constructor".to_owned());
    };
    let Some(second_ctor) = desc.ctors.get(1)
    else {
        return Some("description consumer check needs two constructors".to_owned());
    };
    if !matches!(first_ctor.code, Code::Unit) || !matches!(second_ctor.code, Code::Unit) {
        return Some("description consumer check needs two nullary unit constructors".to_owned());
    }
    let first = DescValue::new(0_usize.into(), Payload::Unit);
    let first_again = DescValue::new(0_usize.into(), Payload::Unit);
    let second = DescValue::new(1_usize.into(), Payload::Unit);
    if !bool::from(generic_eq(desc, &first, &first_again)) {
        return Some("generic equality rejected equal described values".to_owned());
    }
    if bool::from(generic_eq(desc, &first, &second)) {
        return Some("generic equality conflated distinct constructors".to_owned());
    }
    let first_bytes = serialize_value(desc, &first);
    if first_bytes != serialize_value(desc, &first_again) {
        return Some("generic serialization is not deterministic".to_owned());
    }
    if first_bytes == serialize_value(desc, &second) {
        return Some("generic serialization conflated distinct constructors".to_owned());
    }
    None
}

/// Unit tests for the harness's parser, driver dispatch, expectation
/// evaluators, and value notation — the failure and edge paths the passing
/// corpus examples never exercise.
#[cfg(test)]
mod tests
{
    use gandr_core_checker::grade::Grade;
    use gandr_core_checker::types::ValueType;
    use gandr_core_sequent::machine::run_comp;

    use super::*;
    /// Expected substring in an expectation-evaluator failure message.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct FailureNeedle<'needle>(&'needle str);

    impl<'needle> From<&'needle str> for FailureNeedle<'needle>
    {
        #[inline]
        fn from(value: &'needle str) -> Self
        {
            Self(value)
        }
    }

    impl AsRef<str> for FailureNeedle<'_>
    {
        #[inline]
        fn as_ref(&self) -> &str
        {
            self.0
        }
    }

    impl core::fmt::Display for FailureNeedle<'_>
    {
        #[inline]
        fn fmt(
            &self,
            f: &mut core::fmt::Formatter<'_>,
        ) -> core::fmt::Result
        {
            self.0.fmt(f)
        }
    }

    /// A shell program that returns the string `"present"` (model example 24).
    const COND_SRC: &str = "{\n  run probe <- #!{ test -d /tmp; };\n  (if probe.exit_code == 0 \
                            { ret \"present\" } else { ret \"absent\" } : F String)\n}";

    /// A shell program that exits with code 3 (model example 26).
    const EXIT_SRC: &str = "{\n  run missing <- \
                            env.get(\"GANDR_CORPUS_UNSET_VARIABLE_ZZQ\");\n  run code <- (if \
                            string.eq(missing, \"\") { ret 3 } else { ret 7 } : F Integer);\n  \
                            proc.exit(code)\n}";

    /// A program declaring a foreign `sensor` module then calling it in a world
    /// with no handler installed: it blames `perform-no-handler` (example 22).
    const SENSOR_SRC: &str =
        "extern \"c\" from \"sensor\" {\n  def read(channel: i32) -> i64;\n}\n\nsensor.read(0i32)";

    /// A program carrying a `package` attribute on its unit-root definition.
    const META_SRC: &str = "@[package(#{ name = \"acme/parser\", version = \"1.4.0\" })]\ndef \
                            parser_unit = ();";

    #[test]
    fn parse_case_rejects_malformed_directives()
    {
        assert!(
            parse_case(concat!("//", "@ nocolon\n", "ret ()"))
                .unwrap_err()
                .contains("malformed directive"),
            "a directive without `:` is rejected"
        );
        assert!(
            parse_case(concat!(
                "//",
                "@ mode: bogus\n",
                "//",
                "@ expect: clean\n",
                "ret ()"
            ))
            .unwrap_err()
            .contains("unknown mode"),
            "an unknown mode is rejected"
        );
        assert!(
            parse_case(concat!(
                "//",
                "@ mode: session\n",
                "//",
                "@ mode: shell\n",
                "//",
                "@ expect: clean\n",
                "ret ()"
            ))
            .unwrap_err()
            .contains("duplicate `mode`"),
            "a duplicate mode is rejected"
        );
        assert!(
            parse_case(concat!("//", "@ expect: bogus\n", "ret ()"))
                .unwrap_err()
                .contains("unknown expectation"),
            "an unknown expectation is rejected"
        );
        assert!(
            parse_case(concat!(
                "//",
                "@ mode: shell\n",
                "//",
                "@ expect-shell-exit: notanint\n",
                "ret ()"
            ))
            .unwrap_err()
            .contains("non-integer exit code"),
            "a non-integer exit code is rejected"
        );
        assert!(
            parse_case(concat!("//", "@ bogus: x\n", "ret ()"))
                .unwrap_err()
                .contains("unknown directive key"),
            "an unknown directive key is rejected"
        );
        assert!(
            parse_case(concat!(
                "//",
                "@ requires-feature: typo\n",
                "//",
                "@ expect: clean\n",
                "ret ()"
            ))
            .unwrap_err()
            .contains("unknown required feature"),
            "an unknown feature requirement is rejected"
        );
        assert!(
            parse_case("ret ()").unwrap_err().contains("at least one"),
            "an example with no expectation is rejected"
        );
        assert!(
            parse_case(concat!(
                "//",
                "@ mode: desc\n",
                "//",
                "@ expect-desc-store-cells: lots\n",
                "data Bit : Type { Off : Bit; On : Bit; }"
            ))
            .unwrap_err()
            .contains("non-integer stored cell count"),
            "a non-integer stored cell count is rejected"
        );
    }

    #[test]
    fn desc_mode_reports_the_cell_layer_wire()
    {
        // The description → cell store wire, exercised through the harness: a
        // declared single-output `op` lets its `rule` become a cell, and a
        // many-out `op` is declined with an inspectable reason.
        assert!(
            check_case(concat!(
                "data NatId : Type { Zero : NatId; oper id(x : NatId) -> NatId; rule id(Zero) ==> Zero; }\n",
                "//",
                "@ mode: desc\n",
                "//",
                "@ expect-desc-store-cells: 2\n"
            ))
            .is_empty(),
            "one frame cell and one rule cell reach the store"
        );
        assert!(
            check_case(concat!(
                "data NatId : Type { Zero : NatId; oper id(x : NatId) -> NatId; rule id(Zero) ==> Zero; }\n",
                "//",
                "@ mode: desc\n",
                "//",
                "@ expect-desc-store-cells: 9\n"
            ))
            .iter()
            .any(|failure| failure.contains("expected 9 elaborated cell(s)")),
            "a wrong stored-cell count fails with the measured count"
        );
        assert!(
            check_case(concat!(
                "data NatId : Type { Zero : NatId; oper id(x : NatId) -> NatId; }\n",
                "//",
                "@ mode: desc\n",
                "//",
                "@ expect-desc-cell-decline: divmod\n"
            ))
            .iter()
            .any(|failure| failure.contains("(none)")),
            "a decline expectation against a clean elaboration reports no declines"
        );
        assert!(
            check_case(concat!(
                "data NatDiv : Type { Zero : NatDiv; oper divmod(m : NatDiv, n : NatDiv) -> (q : NatDiv, r : NatDiv); }\n",
                "//",
                "@ mode: desc\n",
                "//",
                "@ expect-desc-cell-decline: many-out\n"
            ))
            .is_empty(),
            "the many-out operation's decline is matched by substring"
        );
    }

    #[test]
    fn parse_case_defaults_and_collects()
    {
        let parsed = parse_case(concat!(
            "//",
            "@ requires-feature: regex\n",
            "//",
            "@ expect: clean\n",
            "//",
            "@ expect-def: x\n",
            "ret ()"
        ))
        .expect("well-formed directives parse");
        assert_eq!(Mode::Session, parsed.mode, "the default mode is session");
        assert_eq!(
            parsed.required_features,
            vec![RequiredFeature::Regex],
            "feature requirements are collected in directive order"
        );
        assert_eq!(
            parsed.expects,
            vec![Expect::Clean, Expect::Def("x".to_owned())],
            "expectations are collected in directive order"
        );
        let shell = parse_case(concat!(
            "//",
            "@ mode: shell\n",
            "//",
            "@ expect-shell-exit: 3\n",
            "ret ()"
        ))
        .expect("a shell example parses");
        assert_eq!(Mode::Shell, shell.mode, "the shell mode parses");
        assert_eq!(
            shell.expects,
            vec![Expect::ShellExit(3)],
            "a negative-free integer exit parses"
        );
    }

    #[test]
    fn check_case_dispatches_and_reports_edge_paths()
    {
        assert!(
            check_case(concat!("//", "@ mode: bogus\n", "ret ()"))
                .iter()
                .any(|failure| failure.contains("directive error")),
            "a directive error is surfaced as a single failure"
        );
        assert!(
            check_case(concat!("// only a comment\n", "//", "@ expect: clean"))
                .iter()
                .any(|failure| failure.contains("no top-level items")),
            "a comment-only session example reports no items"
        );
        assert!(
            check_case(concat!(
                "//",
                "@ mode: lower-only\n",
                "//",
                "@ expect: lowers\n",
                "ret ()"
            ))
            .is_empty(),
            "a lower-only example that lowers passes"
        );
        assert!(
            check_case(concat!(
                "//",
                "@ mode: lower-only\n",
                "//",
                "@ expect: clean\n",
                "ret ()"
            ))
            .iter()
            .any(|failure| failure.contains("not valid in lower-only mode")),
            "a non-lowers directive is invalid in lower-only mode"
        );
        #[cfg(not(feature = "ffi"))]
        assert!(
            check_case(concat!(
                "//",
                "@ mode: ffi\n",
                "//",
                "@ expect-ffi-value: ()"
            ))
            .is_empty(),
            "ffi mode is a clean skip when the `ffi` feature is disabled"
        );
        #[cfg(not(feature = "ffi"))]
        assert!(
            check_case(concat!(
                "//",
                "@ requires-feature: ffi\n",
                "//",
                "@ expect: clean\n",
                "not valid gandr"
            ))
            .is_empty(),
            "disabled feature requirements skip before parsing source"
        );
    }

    #[test]
    fn render_value_covers_the_notation()
    {
        assert_eq!("()", render_value(&Value::Unit, 0), "unit");
        assert_eq!("42", render_value(&Value::int(42), 0_usize), "integer");
        assert_eq!("\"hi\"", render_value(&Value::string("hi"), 0), "string");
        assert_eq!("7u32", render_value(&Value::u32(7), 0), "u32");
        assert_eq!("7u64", render_value(&Value::u64(7), 0_usize), "u64");
        assert_eq!("3i32", render_value(&Value::i32(3_i32), 0), "i32");
        assert_eq!("3i64", render_value(&Value::i64(3), 0), "i64");
        assert_eq!("1.5f32", render_value(&Value::f32(1.5), 0), "f32");
        assert_eq!("2.5f64", render_value(&Value::f64(2.5_f64), 0), "f64");
        assert_eq!(
            "(1, 2)",
            render_value(&Value::pair(Value::int(1), Value::int(2)), 0),
            "eager pair"
        );
        assert_eq!(
            "Inl(())",
            render_value(&Value::inj1(Value::Unit), 0),
            "left injection"
        );
        assert_eq!(
            "Inr(())",
            render_value(&Value::inj2(Value::Unit), 0),
            "right injection"
        );
        assert_eq!(
            "[1, 2]",
            render_value(&Value::list(vec![Value::int(1), Value::int(2)]), 0),
            "list"
        );
        assert_eq!(
            "#{a = 1, b = 2}",
            render_value(
                &Value::record([
                    ("b".to_owned(), Value::int(2)),
                    ("a".to_owned(), Value::int(1)),
                ]),
                0
            ),
            "record iterates in canonical field order"
        );
        assert_eq!(
            "<thunk>",
            render_value(&Value::thunk(Grade::ONE, Comp::ret(Value::Unit)), 0),
            "a thunk renders opaquely"
        );
        assert_eq!(
            "9",
            render_value(&Value::annot(Value::int(9), ValueType::integer()), 0),
            "an annotation is transparent"
        );
        assert_eq!(
            "<var x>",
            render_value(&Value::var("x"), 0),
            "a variable renders its name"
        );
        assert_eq!(
            "<opaque>",
            render_value(&Value::hole(0), 0),
            "an unrecognized value renders opaque"
        );
        assert_eq!(
            "<deep>",
            render_value(&Value::Unit, RENDER_DEPTH_LIMIT),
            "rendering is depth-bounded"
        );
    }

    #[test]
    fn stuck_labels_name_every_reachable_reason()
    {
        let cases: [(Comp, &str); 8] = [
            (
                Comp::app(Comp::ret(Value::Unit), Value::Unit),
                "applied-non-function",
            ),
            (
                Comp::bind(
                    Comp::with(Comp::ret(Value::Unit), Comp::ret(Value::Unit)),
                    "x",
                    Comp::ret(Value::var("x")),
                ),
                "sequenced-non-returner",
            ),
            (Comp::force(Value::Unit), "forced-non-thunk"),
            (
                Comp::case(
                    Value::Unit,
                    "l",
                    Comp::ret(Value::var("l")),
                    "r",
                    Comp::ret(Value::var("r")),
                ),
                "cased-non-sum",
            ),
            (
                Comp::split(Value::Unit, "a", "b", Comp::ret(Value::var("a"))),
                "split-non-product",
            ),
            (
                Comp::record_proj(Value::Unit, "f"),
                "record-proj-non-record",
            ),
            (
                Comp::record_proj(Value::record([("a".to_owned(), Value::Unit)]), "z"),
                "record-proj-missing-field",
            ),
            (
                Comp::list_case(
                    Value::Unit,
                    Comp::ret(Value::Unit),
                    "h",
                    "t",
                    Comp::ret(Value::var("h")),
                ),
                "list-cased-non-list",
            ),
        ];
        for (comp, expected) in cases {
            let reason = stuck_reason(&comp);
            assert_eq!(
                stuck_label(&reason).as_ref(),
                expected,
                "stuck label for `{expected}`"
            );
        }
    }

    #[test]
    fn eval_and_blame_labels_name_outcomes()
    {
        assert_eq!(
            "ret 5",
            eval_label(&eval_comp(&Comp::ret(Value::int(5)))),
            "a returner labels its value"
        );
        assert_eq!(
            "non-ret terminal",
            eval_label(&eval_comp(&Comp::with(
                Comp::ret(Value::Unit),
                Comp::ret(Value::Unit)
            ))),
            "a lazy pair is a non-ret terminal"
        );
        assert_eq!(
            "blame:hole",
            eval_label(&eval_comp(&Comp::force(Value::hole(0)))),
            "forcing a hole blames"
        );
        assert_eq!(
            "stuck:forced-non-thunk",
            eval_label(&eval_comp(&Comp::force(Value::Unit))),
            "forcing a non-thunk gets stuck"
        );

        let hole_blame = session_run(&["list.set([1, 2, 3], 9, 0)"]);
        let hole_eval = last_expression(&hole_blame).expect("a last expression");
        assert!(
            matches!(
                *hole_eval,
                Eval::Blame(ref blame) if blame_label(blame).as_ref() == "hole"
            ),
            "an out-of-bounds list update blames `hole`"
        );
        let perform_blame = session_run(&[SENSOR_SRC]);
        let perform_eval = last_expression(&perform_blame).expect("a last expression");
        assert!(
            matches!(
                *perform_eval,
                Eval::Blame(ref blame)
                    if blame_label(blame).as_ref() == "perform-no-handler"
            ),
            "an unhandled foreign call blames `perform-no-handler`"
        );
    }

    #[test]
    fn outcome_labels_name_each_variant()
    {
        assert_eq!(
            "definition",
            outcome_label(&first_outcome(&session_run(&["def d = ();"]))).as_ref(),
            "a definition"
        );
        assert_eq!(
            "expression",
            outcome_label(&first_outcome(&session_run(&["ret 42"]))).as_ref(),
            "an expression"
        );
        assert_eq!(
            "type-error",
            outcome_label(&first_outcome(&session_run(&["[1, 2, 3]"]))).as_ref(),
            "a type error"
        );
        assert_eq!(
            "holey",
            outcome_label(&first_outcome(&session_run(&["{ }"]))).as_ref(),
            "a holey item"
        );
    }

    #[test]
    fn session_failure_evaluates_value_and_definition_expectations()
    {
        let value = session_run(&["ret 42"]);
        assert_pass(session_failure(&value, &Expect::Clean));
        assert_pass(session_failure(&value, &Expect::LastValue("42".to_owned())));
        assert_fail(
            session_failure(&value, &Expect::LastValue("9".to_owned())),
            "last value mismatch",
        );
        assert_fail(
            session_failure(&value, &Expect::Goal),
            "expected at least one goal",
        );
        assert_fail(
            session_failure(&value, &Expect::Diagnostic("x".to_owned())),
            "(none)",
        );
        assert_fail(
            session_failure(&value, &Expect::Attribute("x".to_owned())),
            "(none)",
        );
        assert_fail(
            session_failure(&value, &Expect::Lowers),
            "not valid in session mode",
        );
        assert_fail(
            session_failure(&value, &Expect::Stuck("cased-non-sum".to_owned())),
            "expected stuck",
        );
        assert_fail(
            session_failure(&value, &Expect::Blame("hole".to_owned())),
            "expected blame",
        );

        let definition = session_run(&["def d = ();"]);
        assert_pass(session_failure(&definition, &Expect::Def("d".to_owned())));
        assert_fail(
            session_failure(&definition, &Expect::Def("nope".to_owned())),
            "bound definition of `nope`",
        );
        assert_fail(
            session_failure(&definition, &Expect::LastValue("x".to_owned())),
            "none found",
        );
        assert_fail(
            session_failure(&definition, &Expect::Stuck("x".to_owned())),
            "none found",
        );
        assert_fail(
            session_failure(&definition, &Expect::Blame("x".to_owned())),
            "none found",
        );
    }

    #[test]
    fn session_failure_evaluates_diagnostic_goal_and_attribute_expectations()
    {
        let diagnostic = session_run(&["nowhere"]);
        assert_pass(session_failure(
            &diagnostic,
            &Expect::Diagnostic("nowhere".to_owned()),
        ));
        assert_fail(
            session_failure(&diagnostic, &Expect::Diagnostic("zzz".to_owned())),
            "nowhere",
        );
        assert_fail(session_failure(&diagnostic, &Expect::Clean), "diagnostics");

        let goal = session_run(&["{ }"]);
        assert_pass(session_failure(&goal, &Expect::Goal));
        assert_fail(session_failure(&goal, &Expect::Clean), "goal");

        let attributed = session_run(&[META_SRC]);
        assert_pass(session_failure(
            &attributed,
            &Expect::Attribute("package".to_owned()),
        ));
        assert_fail(
            session_failure(&attributed, &Expect::Attribute("zzz".to_owned())),
            "package",
        );
    }

    #[test]
    fn session_failure_evaluates_blame_and_non_value_last_expressions()
    {
        let blame = session_run(&["list.set([1, 2, 3], 9, 0)"]);
        assert_pass(session_failure(&blame, &Expect::Blame("hole".to_owned())));
        assert_fail(
            session_failure(&blame, &Expect::Blame("perform-no-handler".to_owned())),
            "expected blame",
        );
        assert_fail(
            session_failure(&blame, &Expect::Stuck("x".to_owned())),
            "expected stuck",
        );
        assert_fail(
            session_failure(&blame, &Expect::LastValue("x".to_owned())),
            "did not",
        );
        assert_fail(
            session_failure(&blame, &Expect::Clean),
            "did not terminate with a value",
        );

        let perform = session_run(&[SENSOR_SRC]);
        assert_pass(session_failure(
            &perform,
            &Expect::Blame("perform-no-handler".to_owned()),
        ));
    }

    #[test]
    fn clean_failure_reports_each_kind_of_impurity()
    {
        assert_pass(clean_failure(&session_run(&["ret 42"])));
        assert_fail(clean_failure(&session_run(&["nowhere"])), "diagnostics");
        assert_fail(clean_failure(&session_run(&["{ }"])), "goal");
        let synthetic = SessionRun {
            diagnostics: Vec::new(),
            goals: 0,
            outcomes: vec![first_outcome(&session_run(&["{ }"]))],
            attributes: Vec::new(),
        };
        assert_fail(clean_failure(&synthetic), "holey");
    }

    #[test]
    fn shell_failure_evaluates_every_shell_expectation()
    {
        let ok_value = run_shell_source(COND_SRC);
        let exited = run_shell_source(EXIT_SRC);
        let errored = run_shell_source("#!{ printf 'x' | cat; }");
        let printed = run_shell_source("#!{ printf 'hello'; }");

        assert_pass(shell_failure(
            &errored,
            &Expect::ShellError("pipeline".to_owned()),
        ));
        assert_fail(
            shell_failure(&errored, &Expect::ShellError("zzz".to_owned())),
            "expected a shell error",
        );
        assert_fail(
            shell_failure(&ok_value, &Expect::ShellError("x".to_owned())),
            "the run was prepared",
        );

        assert_pass(shell_failure(&ok_value, &Expect::Clean));
        assert_fail(shell_failure(&exited, &Expect::Clean), "return a value");
        assert_fail(shell_failure(&errored, &Expect::Clean), "failed to prepare");

        assert_pass(shell_failure(
            &ok_value,
            &Expect::ShellValue("\"present\"".to_owned()),
        ));
        assert_fail(
            shell_failure(&ok_value, &Expect::ShellValue("\"x\"".to_owned())),
            "shell value mismatch",
        );
        assert_fail(
            shell_failure(&exited, &Expect::ShellValue("x".to_owned())),
            "did not return a value",
        );
        assert_fail(
            shell_failure(&errored, &Expect::ShellValue("x".to_owned())),
            "failed to prepare",
        );

        assert_pass(shell_failure(&exited, &Expect::ShellExit(3)));
        assert_fail(
            shell_failure(&exited, &Expect::ShellExit(9)),
            "exited with 3",
        );
        assert_fail(
            shell_failure(&ok_value, &Expect::ShellExit(3)),
            "completed differently",
        );
        assert_fail(
            shell_failure(&errored, &Expect::ShellExit(3)),
            "failed to prepare",
        );

        assert_pass(shell_failure(
            &printed,
            &Expect::StdoutContains("hello".to_owned()),
        ));
        assert_fail(
            shell_failure(&printed, &Expect::StdoutContains("zzz".to_owned())),
            "expected stdout to contain",
        );
        assert_fail(
            shell_failure(&ok_value, &Expect::StdoutContains("x".to_owned())),
            "no `stdout` field",
        );
        assert_fail(
            shell_failure(&errored, &Expect::StdoutContains("x".to_owned())),
            "failed to prepare",
        );

        assert_fail(
            shell_failure(&ok_value, &Expect::Goal),
            "not valid in shell mode",
        );
    }

    /// Asserts an expectation evaluator returned a pass (no failure message).
    fn assert_pass(result: Option<String>)
    {
        if let Some(message) = result {
            panic!("expected a pass, got failure: {message}");
        }
    }

    /// Asserts an expectation evaluator failed with a message containing
    /// `needle`.
    fn assert_fail<'needle>(
        result: Option<String>,
        needle: impl Into<FailureNeedle<'needle>>,
    )
    {
        let needle = needle.into();
        match result {
            | Some(message) => assert!(
                message.contains(needle.as_ref()),
                "expected `{needle}` in failure `{message}`"
            ),
            | None => panic!("expected a failure containing `{needle}`, got a pass"),
        }
    }

    /// The first outcome of `run`, panicking when the run produced none.
    fn first_outcome(run: &SessionRun) -> ItemOutcome
    {
        run.outcomes
            .first()
            .cloned()
            .expect("the run produced at least one outcome")
    }

    /// Accumulates a session run over `items`, submitting each to one session —
    /// the same accumulation [`check_session`] performs, exposed so tests can
    /// build real [`SessionRun`] fixtures.
    fn session_run<I>(items: I) -> SessionRun
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut session = Session::new();
        let mut run = SessionRun {
            diagnostics: Vec::new(),
            goals: 0,
            outcomes: Vec::new(),
            attributes: Vec::new(),
        };
        for item in items {
            let item = PipelineSource::from(item.as_ref());
            match session.submit(item) {
                | Ok(submission) => {
                    for diagnostic in &submission.report.diagnostics {
                        run.diagnostics.push(diagnostic.message.clone());
                    }
                    run.goals = run.goals.saturating_add(submission.report.goals.len());
                    for attribute in &submission.report.attributes {
                        run.attributes.push(attribute.schema.clone());
                    }
                    run.outcomes.extend(submission.outcomes.iter().cloned());
                },
                | Err(error) => {
                    panic!("submitting `{}` failed: {error}", item.as_ref());
                },
            }
        }
        run
    }

    /// Runs one borrowed computation on the L machine.
    fn eval_comp(comp: &Comp) -> Eval
    {
        run_comp(comp)
    }

    /// The [`StuckReason`] `comp` gets stuck on under [`eval_comp`].
    fn stuck_reason(comp: &Comp) -> StuckReason
    {
        match eval_comp(comp) {
            | Eval::Stuck(reason) => reason,
            | _ => panic!("expected a stuck evaluation"),
        }
    }
}
