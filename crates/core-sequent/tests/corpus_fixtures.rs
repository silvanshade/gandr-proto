//! The pre-lowered corpus fixture reader (B1 sequent-machines exit gate).
//!
//! The corpus differential and totality sweeps compare / focus every top-level
//! item of every model and pathological corpus program. In the source tree
//! those items came from lowering the `.gandr` sources through the whole
//! front-end (`gandr_pipeline::lower::lower_source_total`). That front-end is
//! outside the B1 machine-port scope, so its output was captured **once**, in
//! the source tree where the pipeline still builds, into the checked-in
//! s-expression fixtures under `tests/fixtures/corpus/{model,pathological}/`.
//! Each fixture carries a provenance header (source-relative path, the BLAKE3
//! digest of the `.gandr` source, and the lowering entry point) and the
//! textual encoding of that file's lowered `Vec<Term>`.
//!
//! This reader is **test-only** and touches no frozen-core code: it parses the
//! fixtures back into [`gandr_core_checker::syntax::Term`]s through the public
//! constructors, with no serde on the AST. The encoding is a deterministic,
//! reviewable s-expression covering exactly the frozen B1 corpus surface; the
//! `panic!` arms mark the extension boundary.
//!
//! Forward pointer: when the surface corpus itself ports (the firewalled
//! front-end), the two sweeps re-point again at live lowering (or regenerated
//! fixtures) and this reader retires; the provenance headers make each fixture
//! traceable to its `.gandr` source until then.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::missing_assert_message,
        clippy::missing_asserts_for_indexing,
        clippy::panic,
        clippy::pattern_type_mismatch,
        clippy::unwrap_used,
        reason = "an iterative reader over trusted, self-generated fixtures: \
                  the positional slice indexing, tag-shape assertions, and \
                  by-ref enum-match binding modes are intrinsic to the compact \
                  decoder (docs/workflow/rust.md)"
    )
)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use gandr_core_checker::effect::EffectOp;
use gandr_core_checker::effect::EffectRow;
use gandr_core_checker::effect::EffectSig;
use gandr_core_checker::grade::Grade;
use gandr_core_checker::prim::NativePrim;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::NumLit;
use gandr_core_checker::syntax::OpClause;
use gandr_core_checker::syntax::Side;
use gandr_core_checker::syntax::SplitMotive;
use gandr_core_checker::syntax::Term;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::syntax::WalkBase;
use gandr_core_checker::syntax::WalkMotive;
use gandr_core_checker::types::CompType;
use gandr_core_checker::types::DataId;
use gandr_core_checker::types::ValueType;

/// One corpus fixture: the provenance source path plus its lowered items.
pub struct Fixture
{
    /// The absolute path of the `.sexp` fixture this was read from, used by the
    /// outcome-snapshot sweep to locate the sibling `.outcome` record and to
    /// digest the fixture bytes for the snapshot's provenance guard.
    pub path: PathBuf,
    /// The source-relative `.gandr` path (from the fixture provenance header),
    /// used verbatim in the sweeps' diagnostics so their output matches the
    /// pre-fixture spelling.
    pub source: String,
    /// The lowered top-level items, in file order.
    pub items: Vec<Term>,
}

/// Reads every fixture under one corpus tree (`"model"` / `"pathological"`),
/// sorted by path for determinism.
pub fn read_tree(tree: &str) -> Vec<Fixture>
{
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/corpus")
        .join(tree);
    let mut fixtures = Vec::new();
    for path in sexp_files(&root) {
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read `{}`: {error}", path.display()));
        fixtures.push(parse_fixture(&text, &path));
    }
    fixtures
}

/// A BLAKE3 fixture-provenance digest over one or more corpus trees — the
/// exact-fixture guard shared by the single-manifest sweeps (the S1 partition
/// and the export exit gate) that fold **many** fixtures into **one** manifest.
///
/// It folds, in the deterministic sorted read order, each fixture's
/// source-relative path and the BLAKE3 of its `.sexp` bytes, so any change to a
/// fixture's bytes, its path, or the set membership changes the digest and
/// forces a re-bless. This is the `corpus_differential` per-file `sexp-b3sum`
/// guard (which pins one fixture per sibling record) generalized to the whole
/// consumed set, so a single header line guards every fixture a sweep reads.
pub fn corpus_fixtures_b3sum(trees: &[&str]) -> String
{
    let mut hasher = blake3::Hasher::new();
    for tree in trees {
        for fixture in &read_tree(tree) {
            let bytes = fs::read(&fixture.path).unwrap_or_else(|error| {
                panic!("cannot read `{}`: {error}", fixture.path.display())
            });
            hasher.update(fixture.source.as_bytes());
            hasher.update(&[0x00]);
            hasher.update(blake3::hash(&bytes).to_hex().as_bytes());
            hasher.update(&[0x0a]);
        }
    }
    String::from(hasher.finalize().to_hex().as_str())
}

/// Collects every `.sexp` fixture under `dir`, recursively, sorted.
fn sexp_files(dir: &Path) -> Vec<PathBuf>
{
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = fs::read_dir(&current)
            .unwrap_or_else(|error| panic!("cannot read `{}`: {error}", current.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                pending.push(path);
            }
            else if path.extension().is_some_and(|ext| ext == "sexp") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Parses one fixture's text into a [`Fixture`], reading the `; source:`
/// provenance line and every top-level term form.
fn parse_fixture(
    text: &str,
    path: &Path,
) -> Fixture
{
    let source = text
        .lines()
        .find_map(|line| line.strip_prefix("; source:"))
        .map_or_else(|| path.display().to_string(), |rest| rest.trim().to_owned());
    let items = parse_all(text).iter().map(decode_term).collect();
    Fixture {
        path: path.to_path_buf(),
        source,
        items,
    }
}

// ------------------------------------------------------------------ Sexp

/// A parsed s-expression: a bare symbol, a quoted string, or a list.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Sexp
{
    /// A bare symbol (a tag or a number rendered as text).
    Sym(String),
    /// A quoted, escaped string payload.
    Str(String),
    /// A parenthesized list of sub-forms.
    List(Vec<Self>),
}

/// Lexes fixture text into a flat token stream, skipping `;` line comments;
/// `(` / `)` become `Sym("(")` / `Sym(")")` sentinels the reader consumes.
fn tokenize(input: &str) -> Vec<Sexp>
{
    let mut toks = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&ch) = chars.peek() {
        match ch {
            | c if c.is_whitespace() => {
                chars.next();
            },
            | ';' => {
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c == '\n' {
                        break;
                    }
                }
            },
            | '(' => {
                chars.next();
                toks.push(Sexp::Sym("(".to_owned()));
            },
            | ')' => {
                chars.next();
                toks.push(Sexp::Sym(")".to_owned()));
            },
            | '"' => {
                chars.next();
                toks.push(Sexp::Str(read_string(&mut chars)));
            },
            | _ => {
                let mut buf = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || c == '(' || c == ')' || c == ';' || c == '"' {
                        break;
                    }
                    buf.push(c);
                    chars.next();
                }
                toks.push(Sexp::Sym(buf));
            },
        }
    }
    toks
}

/// Reads a quoted string body (the opening `"` already consumed), decoding
/// `\\`, `\"`, `\n`, `\t`, `\r`, and `\u{HEX}` escapes.
fn read_string(chars: &mut core::iter::Peekable<core::str::Chars<'_>>) -> String
{
    let mut buf = String::new();
    while let Some(c) = chars.next() {
        if c == '"' {
            break;
        }
        if c == '\\' {
            match chars.next() {
                | Some('n') => buf.push('\n'),
                | Some('t') => buf.push('\t'),
                | Some('r') => buf.push('\r'),
                | Some('\\') => buf.push('\\'),
                | Some('"') => buf.push('"'),
                | Some('u') => buf.push(read_unicode_escape(chars)),
                | Some(other) => buf.push(other),
                | None => break,
            }
        }
        else {
            buf.push(c);
        }
    }
    buf
}

/// Reads a `\u{HEX}` escape body (the `u` already consumed).
fn read_unicode_escape(chars: &mut core::iter::Peekable<core::str::Chars<'_>>) -> char
{
    let mut hex = String::new();
    if chars.peek() == Some(&'{') {
        chars.next();
        while let Some(&c) = chars.peek() {
            chars.next();
            if c == '}' {
                break;
            }
            hex.push(c);
        }
    }
    let cp = u32::from_str_radix(&hex, 16).expect("hex escape");
    char::from_u32(cp).expect("valid scalar")
}

/// Parses fixture text into its sequence of top-level s-expression forms.
///
/// # Contract
///
/// - requires: `input` is trusted fixture text containing balanced
///   s-expressions.
/// - ensures: returns every top-level form and nested list element exactly once
///   in source order.
/// - provides: the parsed s-expression forest consumed by the checker decoder.
/// - panics: unmatched closing or opening parentheses.
/// - intension: tokenization is finite and each token is consumed once by
///   explicit list frames.
///
/// # Adequacy
///
/// - hypothesis: L3 pointwise — an exact nested parse tree distinguishes
///   missing, duplicated, or reordered top-level and child forms.
/// - witness: `fixture_reader_contracts::parse_all_preserves_nested_and_top_level_order`.
fn parse_all(input: &str) -> Vec<Sexp>
{
    let mut forms = Vec::new();
    let mut lists: Vec<Vec<Sexp>> = Vec::new();
    for token in tokenize(input) {
        match token {
            | Sexp::Sym(ref symbol) if symbol == "(" => lists.push(Vec::new()),
            | Sexp::Sym(ref symbol) if symbol == ")" => {
                let Some(items) = lists.pop()
                else {
                    panic!("unexpected )");
                };
                let form = Sexp::List(items);
                if let Some(parent) = lists.last_mut() {
                    parent.push(form);
                }
                else {
                    forms.push(form);
                }
            },
            | form => {
                if let Some(parent) = lists.last_mut() {
                    parent.push(form);
                }
                else {
                    forms.push(form);
                }
            },
        }
    }
    assert!(lists.is_empty(), "unclosed list");
    forms
}

/// The bare-symbol text of a form, or a panic on the wrong shape.
fn as_sym(s: &Sexp) -> &str
{
    match s {
        | Sexp::Sym(t) => t,
        | other => panic!("expected symbol, got {other:?}"),
    }
}

/// The quoted-string text of a form, or a panic on the wrong shape.
fn as_str(s: &Sexp) -> &str
{
    match s {
        | Sexp::Str(t) => t,
        | other => panic!("expected string, got {other:?}"),
    }
}

/// The elements of a list form, or a panic on the wrong shape.
fn as_list(s: &Sexp) -> &[Sexp]
{
    match s {
        | Sexp::List(v) => v,
        | other => panic!("expected list, got {other:?}"),
    }
}

// ---------------------------------------------------------- interpret

/// Rebuilds a usage grade from `gomega` / `(gfin N)`.
fn de_grade(s: &Sexp) -> Grade
{
    match s {
        | Sexp::Sym(t) if t == "gomega" => Grade::OMEGA,
        | Sexp::List(v) => {
            assert_eq!(as_sym(&v[0]), "gfin");
            Grade::fin(as_sym(&v[1]).parse::<u64>().unwrap().into())
        },
        | other => panic!("bad grade {other:?}"),
    }
}

/// Rebuilds an injection / projection side from `l` / `r`.
fn de_side(s: &Sexp) -> Side
{
    match as_sym(s) {
        | "l" => Side::Fst,
        | "r" => Side::Snd,
        | other => panic!("bad side {other}"),
    }
}

/// Rebuilds a typed numeric literal.
fn de_numlit(s: &Sexp) -> NumLit
{
    let v = as_list(s);
    let n = as_sym(&v[1]);
    match as_sym(&v[0]) {
        | "u32" => NumLit::U32(n.parse().unwrap()),
        | "u64" => NumLit::U64(n.parse().unwrap()),
        | "i32" => NumLit::I32(n.parse().unwrap()),
        | "i64" => NumLit::I64(n.parse().unwrap()),
        | "f32" => NumLit::F32(n.parse().unwrap()),
        | "f64" => NumLit::F64(n.parse().unwrap()),
        | other => panic!("bad numlit {other}"),
    }
}

/// Rebuilds a declared-data nominal id `(did SERIAL "name")`.
fn de_dataid(s: &Sexp) -> DataId
{
    let v = as_list(s);
    assert_eq!(as_sym(&v[0]), "did");
    let serial: u64 = as_sym(&v[1]).parse().unwrap();
    DataId::new(serial, as_str(&v[2]))
}

/// Rebuilds a native primitive tag. Corpus source terms only ever carry the
/// argument-free `RecordUpdate`; the panic marks the extension boundary.
fn de_prim(s: &Sexp) -> NativePrim
{
    match as_sym(s) {
        | "recordupdate" => NativePrim::RecordUpdate,
        | other => panic!("unhandled prim {other} — extend the reader"),
    }
}

/// One typed request handled by the iterative fixture decoder.
enum DecodeRequest<'fixture>
{
    EffectOp(&'fixture Sexp),
    EffectSig(&'fixture Sexp),
    EffectRow(&'fixture Sexp),
    ValueType(&'fixture Sexp),
    CompType(&'fixture Sexp),
    OptionalValueType(&'fixture Sexp),
    SplitMotive(&'fixture Sexp),
    WalkMotive(&'fixture Sexp),
    WalkBase(&'fixture Sexp),
    OpClause(&'fixture Sexp),
    Value(&'fixture Sexp),
    Comp(&'fixture Sexp),
    Term(&'fixture Sexp),
}

/// A deferred parent constructor whose children are decoded first.
enum DecodeBuild<'fixture>
{
    EffectOp
    {
        name: &'fixture str,
    },
    EffectSig
    {
        name: &'fixture str,
        operation_count: usize,
    },
    EffectRow
    {
        signature_count: usize,
    },
    ValueTypeProd,
    ValueTypeSum,
    ValueTypeList,
    ValueTypeRecord
    {
        fields: &'fixture [Sexp],
    },
    ValueTypeThunk
    {
        grade: Grade,
    },
    ValueTypeStack,
    ValueTypePath,
    ValueTypeData
    {
        id: DataId,
        argument_count: usize,
    },
    CompTypeF,
    CompTypeArrow,
    CompTypeWith,
    OptionalValueTypeSome,
    SplitMotiveSome
    {
        binder: &'fixture str,
    },
    WalkMotive
    {
        value_binder: &'fixture str,
        path_binder: &'fixture str,
        proof_binder: &'fixture str,
    },
    WalkBase
    {
        binder: &'fixture str,
    },
    OpClause
    {
        operation: &'fixture str,
        payload: &'fixture str,
        resume: &'fixture str,
    },
    ValuePair,
    ValueInjection
    {
        side: Side,
    },
    ValueList
    {
        element_count: usize,
    },
    ValueRecord
    {
        fields: &'fixture [Sexp],
    },
    ValueThunk
    {
        grade: Grade,
    },
    ValueAnnotation,
    ValueHere,
    ValueConstructor
    {
        id: DataId,
        tag: usize,
    },
    CompAbstraction
    {
        binder: &'fixture str,
    },
    CompApplication,
    CompReturn,
    CompBind
    {
        binder: &'fixture str,
    },
    CompForce,
    CompCase
    {
        left_binder: &'fixture str,
        right_binder: &'fixture str,
    },
    CompDataCase
    {
        arms: &'fixture [Sexp],
    },
    CompListCase
    {
        head_binder: &'fixture str,
        tail_binder: &'fixture str,
    },
    CompSplit
    {
        first_binder: &'fixture str,
        second_binder: &'fixture str,
    },
    CompRecordProjection
    {
        label: &'fixture str,
    },
    CompWith,
    CompProjection
    {
        side: Side,
    },
    CompDup,
    CompDrop,
    CompPerform
    {
        operation: &'fixture str,
    },
    CompHandle
    {
        return_binder: &'fixture str,
        operation_count: usize,
    },
    CompResume,
    CompReset,
    CompShift
    {
        resume_binder: &'fixture str,
    },
    CompNative
    {
        primitive: NativePrim,
        argument_count: usize,
    },
    CompWalk,
    TermValue,
    TermComp,
}

/// One pending decoder action.
enum DecodeTask<'fixture>
{
    Request(DecodeRequest<'fixture>),
    Build(DecodeBuild<'fixture>),
}

/// A completed child value awaiting its parent constructor.
enum Decoded
{
    EffectOp(EffectOp),
    EffectSig(EffectSig),
    EffectRow(EffectRow),
    ValueType(ValueType),
    CompType(CompType),
    OptionalValueType(Option<Rc<ValueType>>),
    SplitMotive(Option<Box<SplitMotive>>),
    WalkMotive(WalkMotive),
    WalkBase(WalkBase),
    OpClause(OpClause),
    Value(Value),
    Comp(Comp),
    Term(Term),
}

/// Define one checked projection from the decoder's heterogeneous value stack.
macro_rules! decoded_projection {
    ($method:ident, $variant:ident, $output:ty, $panic:literal) => {
        /// Extract the typed result expected by this projection.
        ///
        /// # Contract
        ///
        /// - ensures: returns the enclosed child when `self` has this projection's
        ///   declared variant.
        /// - provides: the typed child required by the active decoder build.
        /// - panics: when decoder scheduling supplies a different child variant.
        /// - intension: performs one constant-time enum projection.
        ///
        /// # Adequacy
        ///
        /// - hypothesis: L3 residue — the exact stack witness distinguishes projection
        ///   and LIFO drift; both corpus outcome sweeps exercise the generated
        ///   projection family across registered fixture forms.
        /// - witness: `fixture_reader_contracts::pop_decoded_returns_latest_child`.
        /// - witness: `l_machine_matches_the_outcome_snapshots_on_the_model_corpus`.
        /// - witness: `l_machine_matches_the_outcome_snapshots_on_the_pathological_corpus`.
        fn $method(self) -> $output
        {
            let Self::$variant(value) = self
            else {
                panic!($panic);
            };
            value
        }
    };
}

impl Decoded
{
    decoded_projection!(
        into_effect_op,
        EffectOp,
        EffectOp,
        "decoder stack expected an effect operation"
    );
    decoded_projection!(
        into_effect_sig,
        EffectSig,
        EffectSig,
        "decoder stack expected an effect signature"
    );
    decoded_projection!(
        into_effect_row,
        EffectRow,
        EffectRow,
        "decoder stack expected an effect row"
    );
    decoded_projection!(
        into_value_type,
        ValueType,
        ValueType,
        "decoder stack expected a value type"
    );
    decoded_projection!(
        into_comp_type,
        CompType,
        CompType,
        "decoder stack expected a computation type"
    );
    decoded_projection!(
        into_optional_value_type,
        OptionalValueType,
        Option<Rc<ValueType>>,
        "decoder stack expected an optional value type"
    );
    decoded_projection!(
        into_split_motive,
        SplitMotive,
        Option<Box<SplitMotive>>,
        "decoder stack expected a split motive"
    );
    decoded_projection!(
        into_walk_motive,
        WalkMotive,
        WalkMotive,
        "decoder stack expected a Walk motive"
    );
    decoded_projection!(
        into_walk_base,
        WalkBase,
        WalkBase,
        "decoder stack expected a Walk base"
    );
    decoded_projection!(
        into_op_clause,
        OpClause,
        OpClause,
        "decoder stack expected an operation clause"
    );
    decoded_projection!(into_value, Value, Value, "decoder stack expected a value");
    decoded_projection!(
        into_comp,
        Comp,
        Comp,
        "decoder stack expected a computation"
    );
    decoded_projection!(into_term, Term, Term, "decoder stack expected a term");
}

/// Pops one completed child from the decoder's value stack.
///
/// # Contract
///
/// - requires: `completed` contains the children already produced for the
///   active build action.
/// - ensures: removes and returns the most recently completed child while
///   leaving every earlier child in place.
/// - provides: the next typed projection consumed by the active build action.
/// - panics: when the stack is empty because a build ran before its children.
/// - intension: performs one constant-time stack pop.
///
/// # Adequacy
///
/// - hypothesis: L3 pointwise — an exact two-child stack witness distinguishes
///   newest-child selection, removal, and preservation of the earlier child.
/// - witness: `fixture_reader_contracts::pop_decoded_returns_latest_child`.
fn pop_decoded(completed: &mut Vec<Decoded>) -> Decoded
{
    let Some(value) = completed.pop()
    else {
        panic!("decoder build ran before all children completed");
    };
    value
}

/// Rebuilds one fixture term with explicit request and continuation stacks.
///
/// # Contract
///
/// - requires: `root` is trusted fixture syntax rooted at `(v value)` or `(c
///   comp)`.
/// - ensures: for registered model and pathological fixtures, the rebuilt term
///   yields the pinned canonical machine outcome.
/// - provides: one checker [`Term`] for the fixture harness.
/// - panics: malformed tags, fields, or child-stack invariants in trusted
///   input.
/// - intension: every loop step consumes one request/build action and each
///   request schedules only the finite children present in its s-expression;
///   native stack use is independent of term depth.
///
/// # Adequacy
///
/// - hypothesis: L2 pinned-golden — the behavioral contract deliberately makes
///   no syntax-tree identity claim; independent model and pathological outcome
///   snapshots cover every registered fixture.
/// - witness: `l_machine_matches_the_outcome_snapshots_on_the_model_corpus`.
/// - witness: `l_machine_matches_the_outcome_snapshots_on_the_pathological_corpus`.
fn decode_term(root: &Sexp) -> Term
{
    let mut work = vec![DecodeTask::Request(DecodeRequest::Term(root))];
    let mut completed = Vec::new();
    while let Some(task) = work.pop() {
        match task {
            | DecodeTask::Request(request) => match request {
                | DecodeRequest::EffectOp(s) => {
                    let v = as_list(s);
                    assert_eq!(as_sym(&v[0]), "op");
                    work.push(DecodeTask::Build(DecodeBuild::EffectOp {
                        name: as_str(&v[1]),
                    }));
                    work.push(DecodeTask::Request(DecodeRequest::ValueType(&v[3])));
                    work.push(DecodeTask::Request(DecodeRequest::ValueType(&v[2])));
                },
                | DecodeRequest::EffectSig(s) => {
                    let v = as_list(s);
                    assert_eq!(as_sym(&v[0]), "sig");
                    let operations = &v[2 ..];
                    work.push(DecodeTask::Build(DecodeBuild::EffectSig {
                        name: as_str(&v[1]),
                        operation_count: operations.len(),
                    }));
                    for operation in operations.iter().rev() {
                        work.push(DecodeTask::Request(DecodeRequest::EffectOp(operation)));
                    }
                },
                | DecodeRequest::EffectRow(s) => {
                    let v = as_list(s);
                    assert_eq!(as_sym(&v[0]), "row");
                    let signatures = &v[1 ..];
                    work.push(DecodeTask::Build(DecodeBuild::EffectRow {
                        signature_count: signatures.len(),
                    }));
                    for signature in signatures.iter().rev() {
                        work.push(DecodeTask::Request(DecodeRequest::EffectSig(signature)));
                    }
                },
                | DecodeRequest::ValueType(s) => match s {
                    | Sexp::Sym(tag) => {
                        let value_type = match tag.as_str() {
                            | "tunit" => ValueType::Unit,
                            | "tuniverse" => ValueType::Universe,
                            | "tunknown" => ValueType::Unknown,
                            | other => panic!("bad vtype sym {other}"),
                        };
                        completed.push(Decoded::ValueType(value_type));
                    },
                    | Sexp::List(v) => match as_sym(&v[0]) {
                        | "tatom" => completed.push(Decoded::ValueType(ValueType::Atom(
                            as_str(&v[1]).to_owned(),
                        ))),
                        | "tprod" => {
                            work.push(DecodeTask::Build(DecodeBuild::ValueTypeProd));
                            work.push(DecodeTask::Request(DecodeRequest::ValueType(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::ValueType(&v[1])));
                        },
                        | "tsum" => {
                            work.push(DecodeTask::Build(DecodeBuild::ValueTypeSum));
                            work.push(DecodeTask::Request(DecodeRequest::ValueType(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::ValueType(&v[1])));
                        },
                        | "tlist" => {
                            work.push(DecodeTask::Build(DecodeBuild::ValueTypeList));
                            work.push(DecodeTask::Request(DecodeRequest::ValueType(&v[1])));
                        },
                        | "trec" => {
                            let fields = &v[1 ..];
                            work.push(DecodeTask::Build(DecodeBuild::ValueTypeRecord { fields }));
                            for field in fields.iter().rev() {
                                let field = as_list(field);
                                work.push(DecodeTask::Request(DecodeRequest::ValueType(&field[1])));
                            }
                        },
                        | "tthunk" => {
                            work.push(DecodeTask::Build(DecodeBuild::ValueTypeThunk {
                                grade: de_grade(&v[1]),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::CompType(&v[2])));
                        },
                        | "tstk" => {
                            work.push(DecodeTask::Build(DecodeBuild::ValueTypeStack));
                            work.push(DecodeTask::Request(DecodeRequest::CompType(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::CompType(&v[1])));
                        },
                        | "tpath" => {
                            work.push(DecodeTask::Build(DecodeBuild::ValueTypePath));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[3])));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::ValueType(&v[1])));
                        },
                        | "tdata" => {
                            let arguments = &v[2 ..];
                            work.push(DecodeTask::Build(DecodeBuild::ValueTypeData {
                                id: de_dataid(&v[1]),
                                argument_count: arguments.len(),
                            }));
                            for argument in arguments.iter().rev() {
                                work.push(DecodeTask::Request(DecodeRequest::ValueType(argument)));
                            }
                        },
                        | other => panic!("bad vtype list {other}"),
                    },
                    | other => panic!("bad vtype {other:?}"),
                },
                | DecodeRequest::CompType(s) => match s {
                    | Sexp::Sym(tag) if tag == "ctunknown" => {
                        completed.push(Decoded::CompType(CompType::Unknown));
                    },
                    | Sexp::List(v) => match as_sym(&v[0]) {
                        | "ctf" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompTypeF));
                            work.push(DecodeTask::Request(DecodeRequest::EffectRow(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::ValueType(&v[1])));
                        },
                        | "ctarrow" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompTypeArrow));
                            work.push(DecodeTask::Request(DecodeRequest::CompType(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::ValueType(&v[1])));
                        },
                        | "ctwith" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompTypeWith));
                            work.push(DecodeTask::Request(DecodeRequest::CompType(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::CompType(&v[1])));
                        },
                        | other => panic!("bad ctype list {other}"),
                    },
                    | other => panic!("bad ctype {other:?}"),
                },
                | DecodeRequest::OptionalValueType(s) => match s {
                    | Sexp::Sym(tag) if tag == "none" => {
                        completed.push(Decoded::OptionalValueType(None));
                    },
                    | Sexp::List(v) => {
                        assert_eq!(as_sym(&v[0]), "some");
                        work.push(DecodeTask::Build(DecodeBuild::OptionalValueTypeSome));
                        work.push(DecodeTask::Request(DecodeRequest::ValueType(&v[1])));
                    },
                    | other => panic!("bad opt vtype {other:?}"),
                },
                | DecodeRequest::SplitMotive(s) => match s {
                    | Sexp::Sym(tag) if tag == "none" => {
                        completed.push(Decoded::SplitMotive(None));
                    },
                    | Sexp::List(v) => {
                        assert_eq!(as_sym(&v[0]), "some");
                        work.push(DecodeTask::Build(DecodeBuild::SplitMotiveSome {
                            binder: as_str(&v[1]),
                        }));
                        work.push(DecodeTask::Request(DecodeRequest::CompType(&v[2])));
                    },
                    | other => panic!("bad split motive {other:?}"),
                },
                | DecodeRequest::WalkMotive(s) => {
                    let v = as_list(s);
                    assert_eq!(as_sym(&v[0]), "wmotive");
                    work.push(DecodeTask::Build(DecodeBuild::WalkMotive {
                        value_binder: as_str(&v[1]),
                        path_binder: as_str(&v[2]),
                        proof_binder: as_str(&v[3]),
                    }));
                    work.push(DecodeTask::Request(DecodeRequest::CompType(&v[4])));
                },
                | DecodeRequest::WalkBase(s) => {
                    let v = as_list(s);
                    assert_eq!(as_sym(&v[0]), "wbase");
                    work.push(DecodeTask::Build(DecodeBuild::WalkBase {
                        binder: as_str(&v[1]),
                    }));
                    work.push(DecodeTask::Request(DecodeRequest::Comp(&v[2])));
                },
                | DecodeRequest::OpClause(s) => {
                    let v = as_list(s);
                    assert_eq!(as_sym(&v[0]), "opclause");
                    work.push(DecodeTask::Build(DecodeBuild::OpClause {
                        operation: as_str(&v[1]),
                        payload: as_str(&v[2]),
                        resume: as_str(&v[3]),
                    }));
                    work.push(DecodeTask::Request(DecodeRequest::Comp(&v[4])));
                },
                | DecodeRequest::Value(s) => match s {
                    | Sexp::Sym(tag) if tag == "u" => {
                        completed.push(Decoded::Value(Value::Unit));
                    },
                    | Sexp::List(v) => match as_sym(&v[0]) {
                        | "var" => {
                            completed.push(Decoded::Value(Value::Var(as_str(&v[1]).to_owned())));
                        },
                        | "i" => completed
                            .push(Decoded::Value(Value::Int(as_sym(&v[1]).parse().unwrap()))),
                        | "s" => {
                            completed.push(Decoded::Value(Value::Str(as_str(&v[1]).to_owned())));
                        },
                        | "n" => completed.push(Decoded::Value(Value::Num(de_numlit(&v[1])))),
                        | "pair" => {
                            work.push(DecodeTask::Build(DecodeBuild::ValuePair));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "inj" => {
                            work.push(DecodeTask::Build(DecodeBuild::ValueInjection {
                                side: de_side(&v[1]),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[2])));
                        },
                        | "vlist" => {
                            let elements = &v[1 ..];
                            work.push(DecodeTask::Build(DecodeBuild::ValueList {
                                element_count: elements.len(),
                            }));
                            for element in elements.iter().rev() {
                                work.push(DecodeTask::Request(DecodeRequest::Value(element)));
                            }
                        },
                        | "vrec" => {
                            let fields = &v[1 ..];
                            work.push(DecodeTask::Build(DecodeBuild::ValueRecord { fields }));
                            for field in fields.iter().rev() {
                                let field = as_list(field);
                                work.push(DecodeTask::Request(DecodeRequest::Value(&field[1])));
                            }
                        },
                        | "thunk" => {
                            work.push(DecodeTask::Build(DecodeBuild::ValueThunk {
                                grade: de_grade(&v[1]),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[2])));
                        },
                        | "annot" => {
                            work.push(DecodeTask::Build(DecodeBuild::ValueAnnotation));
                            work.push(DecodeTask::Request(DecodeRequest::ValueType(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "vhole" => completed
                            .push(Decoded::Value(Value::Hole(as_sym(&v[1]).parse().unwrap()))),
                        | "here" => {
                            work.push(DecodeTask::Build(DecodeBuild::ValueHere));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "ctor" => {
                            work.push(DecodeTask::Build(DecodeBuild::ValueConstructor {
                                id: de_dataid(&v[1]),
                                tag: as_sym(&v[2]).parse().unwrap(),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[3])));
                        },
                        | other => panic!("bad value list {other}"),
                    },
                    | other => panic!("bad value {other:?}"),
                },
                | DecodeRequest::Comp(s) => {
                    let v = as_list(s);
                    match as_sym(&v[0]) {
                        | "abs" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompAbstraction {
                                binder: as_str(&v[1]),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[3])));
                            work.push(DecodeTask::Request(DecodeRequest::OptionalValueType(&v[2])));
                        },
                        | "app" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompApplication));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[1])));
                        },
                        | "ret" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompReturn));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "bind" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompBind {
                                binder: as_str(&v[2]),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[3])));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[1])));
                        },
                        | "force" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompForce));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "case" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompCase {
                                left_binder: as_str(&v[2]),
                                right_binder: as_str(&v[4]),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[5])));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[3])));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "datacase" => {
                            let arms = &v[2 ..];
                            work.push(DecodeTask::Build(DecodeBuild::CompDataCase { arms }));
                            for arm in arms.iter().rev() {
                                let arm = as_list(arm);
                                assert_eq!(as_sym(&arm[0]), "arm");
                                work.push(DecodeTask::Request(DecodeRequest::Comp(&arm[2])));
                            }
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "listcase" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompListCase {
                                head_binder: as_str(&v[3]),
                                tail_binder: as_str(&v[4]),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[5])));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "split" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompSplit {
                                first_binder: as_str(&v[2]),
                                second_binder: as_str(&v[3]),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[5])));
                            work.push(DecodeTask::Request(DecodeRequest::SplitMotive(&v[4])));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "recordproj" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompRecordProjection {
                                label: as_str(&v[2]),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "cwith" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompWith));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[1])));
                        },
                        | "prj" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompProjection {
                                side: de_side(&v[1]),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[2])));
                        },
                        | "dup" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompDup));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "drop" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompDrop));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "perform" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompPerform {
                                operation: as_str(&v[2]),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[3])));
                            work.push(DecodeTask::Request(DecodeRequest::EffectSig(&v[1])));
                        },
                        | "handle" => {
                            let operations = &v[5 ..];
                            work.push(DecodeTask::Build(DecodeBuild::CompHandle {
                                return_binder: as_str(&v[3]),
                                operation_count: operations.len(),
                            }));
                            for operation in operations.iter().rev() {
                                work.push(DecodeTask::Request(DecodeRequest::OpClause(operation)));
                            }
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[4])));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::EffectSig(&v[1])));
                        },
                        | "resume" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompResume));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "reset" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompReset));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[1])));
                        },
                        | "shift" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompShift {
                                resume_binder: as_str(&v[1]),
                            }));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[2])));
                        },
                        | "chole" => completed
                            .push(Decoded::Comp(Comp::Hole(as_sym(&v[1]).parse().unwrap()))),
                        | "native" => {
                            let arguments = &v[2 ..];
                            work.push(DecodeTask::Build(DecodeBuild::CompNative {
                                primitive: de_prim(&v[1]),
                                argument_count: arguments.len(),
                            }));
                            for argument in arguments.iter().rev() {
                                work.push(DecodeTask::Request(DecodeRequest::Value(argument)));
                            }
                        },
                        | "walk" => {
                            work.push(DecodeTask::Build(DecodeBuild::CompWalk));
                            work.push(DecodeTask::Request(DecodeRequest::WalkBase(&v[3])));
                            work.push(DecodeTask::Request(DecodeRequest::WalkMotive(&v[2])));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | other => panic!("bad comp {other}"),
                    }
                },
                | DecodeRequest::Term(s) => {
                    let v = as_list(s);
                    match as_sym(&v[0]) {
                        | "v" => {
                            work.push(DecodeTask::Build(DecodeBuild::TermValue));
                            work.push(DecodeTask::Request(DecodeRequest::Value(&v[1])));
                        },
                        | "c" => {
                            work.push(DecodeTask::Build(DecodeBuild::TermComp));
                            work.push(DecodeTask::Request(DecodeRequest::Comp(&v[1])));
                        },
                        | other => panic!("bad term {other}"),
                    }
                },
            },
            | DecodeTask::Build(build) => match build {
                | DecodeBuild::EffectOp { name } => {
                    let reply = pop_decoded(&mut completed).into_value_type();
                    let payload = pop_decoded(&mut completed).into_value_type();
                    completed.push(Decoded::EffectOp(EffectOp::new(
                        name.into(),
                        payload,
                        reply,
                    )));
                },
                | DecodeBuild::EffectSig {
                    name,
                    operation_count,
                } => {
                    let mut operations = Vec::with_capacity(operation_count);
                    for _ in 0 .. operation_count {
                        operations.push(pop_decoded(&mut completed).into_effect_op());
                    }
                    operations.reverse();
                    completed.push(Decoded::EffectSig(EffectSig::new(name.into(), operations)));
                },
                | DecodeBuild::EffectRow { signature_count } => {
                    let mut signatures = Vec::with_capacity(signature_count);
                    for _ in 0 .. signature_count {
                        signatures.push(pop_decoded(&mut completed).into_effect_sig());
                    }
                    signatures.reverse();
                    let mut row = EffectRow::EMPTY;
                    for signature in signatures {
                        row = row.union(&EffectRow::singleton(signature));
                    }
                    completed.push(Decoded::EffectRow(row));
                },
                | DecodeBuild::ValueTypeProd => {
                    let right = pop_decoded(&mut completed).into_value_type();
                    let left = pop_decoded(&mut completed).into_value_type();
                    completed.push(Decoded::ValueType(ValueType::Prod(
                        Rc::new(left),
                        Rc::new(right),
                    )));
                },
                | DecodeBuild::ValueTypeSum => {
                    let right = pop_decoded(&mut completed).into_value_type();
                    let left = pop_decoded(&mut completed).into_value_type();
                    completed.push(Decoded::ValueType(ValueType::Sum(
                        Rc::new(left),
                        Rc::new(right),
                    )));
                },
                | DecodeBuild::ValueTypeList => {
                    let element = pop_decoded(&mut completed).into_value_type();
                    completed.push(Decoded::ValueType(ValueType::List(Rc::new(element))));
                },
                | DecodeBuild::ValueTypeRecord { fields } => {
                    let mut values = Vec::with_capacity(fields.len());
                    for _ in 0 .. fields.len() {
                        values.push(pop_decoded(&mut completed).into_value_type());
                    }
                    values.reverse();
                    let mut record = BTreeMap::new();
                    for (field, value) in fields.iter().zip(values) {
                        let field = as_list(field);
                        record.insert(as_str(&field[0]).to_owned(), Rc::new(value));
                    }
                    completed.push(Decoded::ValueType(ValueType::Record(record)));
                },
                | DecodeBuild::ValueTypeThunk { grade } => {
                    let body = pop_decoded(&mut completed).into_comp_type();
                    completed.push(Decoded::ValueType(ValueType::Thunk(grade, Rc::new(body))));
                },
                | DecodeBuild::ValueTypeStack => {
                    let right = pop_decoded(&mut completed).into_comp_type();
                    let left = pop_decoded(&mut completed).into_comp_type();
                    completed.push(Decoded::ValueType(ValueType::Stk(
                        Rc::new(left),
                        Rc::new(right),
                    )));
                },
                | DecodeBuild::ValueTypePath => {
                    let right = pop_decoded(&mut completed).into_value();
                    let left = pop_decoded(&mut completed).into_value();
                    let carrier = pop_decoded(&mut completed).into_value_type();
                    completed.push(Decoded::ValueType(ValueType::path(carrier, left, right)));
                },
                | DecodeBuild::ValueTypeData { id, argument_count } => {
                    let mut arguments = Vec::with_capacity(argument_count);
                    for _ in 0 .. argument_count {
                        arguments.push(pop_decoded(&mut completed).into_value_type());
                    }
                    arguments.reverse();
                    completed.push(Decoded::ValueType(ValueType::data(id, arguments)));
                },
                | DecodeBuild::CompTypeF => {
                    let effects = pop_decoded(&mut completed).into_effect_row();
                    let value_type = pop_decoded(&mut completed).into_value_type();
                    completed.push(Decoded::CompType(CompType::F(Rc::new(value_type), effects)));
                },
                | DecodeBuild::CompTypeArrow => {
                    let body = pop_decoded(&mut completed).into_comp_type();
                    let parameter = pop_decoded(&mut completed).into_value_type();
                    completed.push(Decoded::CompType(CompType::Arrow(
                        Rc::new(parameter),
                        Rc::new(body),
                    )));
                },
                | DecodeBuild::CompTypeWith => {
                    let right = pop_decoded(&mut completed).into_comp_type();
                    let left = pop_decoded(&mut completed).into_comp_type();
                    completed.push(Decoded::CompType(CompType::With(
                        Rc::new(left),
                        Rc::new(right),
                    )));
                },
                | DecodeBuild::OptionalValueTypeSome => {
                    let value_type = pop_decoded(&mut completed).into_value_type();
                    completed.push(Decoded::OptionalValueType(Some(Rc::new(value_type))));
                },
                | DecodeBuild::SplitMotiveSome { binder } => {
                    let body = pop_decoded(&mut completed).into_comp_type();
                    completed.push(Decoded::SplitMotive(Some(Box::new(SplitMotive::new(
                        binder, body,
                    )))));
                },
                | DecodeBuild::WalkMotive {
                    value_binder,
                    path_binder,
                    proof_binder,
                } => {
                    let body = pop_decoded(&mut completed).into_comp_type();
                    completed.push(Decoded::WalkMotive(WalkMotive::new(
                        value_binder,
                        path_binder,
                        proof_binder,
                        body,
                    )));
                },
                | DecodeBuild::WalkBase { binder } => {
                    let body = pop_decoded(&mut completed).into_comp();
                    completed.push(Decoded::WalkBase(WalkBase::new(binder, body)));
                },
                | DecodeBuild::OpClause {
                    operation,
                    payload,
                    resume,
                } => {
                    let body = pop_decoded(&mut completed).into_comp();
                    completed.push(Decoded::OpClause(OpClause::new(
                        operation, payload, resume, body,
                    )));
                },
                | DecodeBuild::ValuePair => {
                    let right = pop_decoded(&mut completed).into_value();
                    let left = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Value(Value::Pair(Rc::new(left), Rc::new(right))));
                },
                | DecodeBuild::ValueInjection { side } => {
                    let value = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Value(Value::Inj(side, Rc::new(value))));
                },
                | DecodeBuild::ValueList { element_count } => {
                    let mut elements = Vec::with_capacity(element_count);
                    for _ in 0 .. element_count {
                        elements.push(Rc::new(pop_decoded(&mut completed).into_value()));
                    }
                    elements.reverse();
                    completed.push(Decoded::Value(Value::List(elements)));
                },
                | DecodeBuild::ValueRecord { fields } => {
                    let mut values = Vec::with_capacity(fields.len());
                    for _ in 0 .. fields.len() {
                        values.push(pop_decoded(&mut completed).into_value());
                    }
                    values.reverse();
                    let mut record = BTreeMap::new();
                    for (field, value) in fields.iter().zip(values) {
                        let field = as_list(field);
                        record.insert(as_str(&field[0]).to_owned(), Rc::new(value));
                    }
                    completed.push(Decoded::Value(Value::Record(record)));
                },
                | DecodeBuild::ValueThunk { grade } => {
                    let body = pop_decoded(&mut completed).into_comp();
                    completed.push(Decoded::Value(Value::Thunk(grade, Rc::new(body))));
                },
                | DecodeBuild::ValueAnnotation => {
                    let value_type = pop_decoded(&mut completed).into_value_type();
                    let value = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Value(Value::Annot(
                        Rc::new(value),
                        Rc::new(value_type),
                    )));
                },
                | DecodeBuild::ValueHere => {
                    let value = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Value(Value::Here(Rc::new(value))));
                },
                | DecodeBuild::ValueConstructor { id, tag } => {
                    let payload = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Value(Value::Ctor {
                        id,
                        tag,
                        payload: Rc::new(payload),
                    }));
                },
                | DecodeBuild::CompAbstraction { binder } => {
                    let body = pop_decoded(&mut completed).into_comp();
                    let annotation = pop_decoded(&mut completed).into_optional_value_type();
                    completed.push(Decoded::Comp(Comp::Abs(
                        binder.to_owned(),
                        annotation,
                        Rc::new(body),
                    )));
                },
                | DecodeBuild::CompApplication => {
                    let argument = pop_decoded(&mut completed).into_value();
                    let function = pop_decoded(&mut completed).into_comp();
                    completed.push(Decoded::Comp(Comp::App(
                        Rc::new(function),
                        Rc::new(argument),
                    )));
                },
                | DecodeBuild::CompReturn => {
                    let value = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Comp(Comp::Ret(Rc::new(value))));
                },
                | DecodeBuild::CompBind { binder } => {
                    let body = pop_decoded(&mut completed).into_comp();
                    let scrutinee = pop_decoded(&mut completed).into_comp();
                    completed.push(Decoded::Comp(Comp::Bind(
                        Rc::new(scrutinee),
                        binder.to_owned(),
                        Rc::new(body),
                    )));
                },
                | DecodeBuild::CompForce => {
                    let value = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Comp(Comp::Force(Rc::new(value))));
                },
                | DecodeBuild::CompCase {
                    left_binder,
                    right_binder,
                } => {
                    let right_body = pop_decoded(&mut completed).into_comp();
                    let left_body = pop_decoded(&mut completed).into_comp();
                    let scrutinee = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Comp(Comp::Case(
                        Rc::new(scrutinee),
                        (left_binder.to_owned(), Rc::new(left_body)),
                        (right_binder.to_owned(), Rc::new(right_body)),
                    )));
                },
                | DecodeBuild::CompDataCase { arms } => {
                    let mut bodies = Vec::with_capacity(arms.len());
                    for _ in 0 .. arms.len() {
                        bodies.push(pop_decoded(&mut completed).into_comp());
                    }
                    bodies.reverse();
                    let scrutinee = pop_decoded(&mut completed).into_value();
                    let decoded_arms = arms
                        .iter()
                        .zip(bodies)
                        .map(|(arm, body)| {
                            let arm = as_list(arm);
                            (as_str(&arm[1]).to_owned(), Rc::new(body))
                        })
                        .collect();
                    completed.push(Decoded::Comp(Comp::DataCase(
                        Rc::new(scrutinee),
                        decoded_arms,
                    )));
                },
                | DecodeBuild::CompListCase {
                    head_binder,
                    tail_binder,
                } => {
                    let cons = pop_decoded(&mut completed).into_comp();
                    let nil = pop_decoded(&mut completed).into_comp();
                    let scrutinee = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Comp(Comp::ListCase {
                        scrut: Rc::new(scrutinee),
                        nil: Rc::new(nil),
                        head: head_binder.to_owned(),
                        tail: tail_binder.to_owned(),
                        cons: Rc::new(cons),
                    }));
                },
                | DecodeBuild::CompSplit {
                    first_binder,
                    second_binder,
                } => {
                    let body = pop_decoded(&mut completed).into_comp();
                    let motive = pop_decoded(&mut completed).into_split_motive();
                    let scrutinee = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Comp(Comp::Split {
                        scrut: Rc::new(scrutinee),
                        fst_name: first_binder.to_owned(),
                        snd_name: second_binder.to_owned(),
                        motive,
                        body: Rc::new(body),
                    }));
                },
                | DecodeBuild::CompRecordProjection { label } => {
                    let record = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Comp(Comp::RecordProj {
                        record: Rc::new(record),
                        label: label.to_owned(),
                    }));
                },
                | DecodeBuild::CompWith => {
                    let right = pop_decoded(&mut completed).into_comp();
                    let left = pop_decoded(&mut completed).into_comp();
                    completed.push(Decoded::Comp(Comp::With(Rc::new(left), Rc::new(right))));
                },
                | DecodeBuild::CompProjection { side } => {
                    let comp = pop_decoded(&mut completed).into_comp();
                    completed.push(Decoded::Comp(Comp::Prj(side, Rc::new(comp))));
                },
                | DecodeBuild::CompDup => {
                    let value = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Comp(Comp::Dup(Rc::new(value))));
                },
                | DecodeBuild::CompDrop => {
                    let value = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Comp(Comp::Drop(Rc::new(value))));
                },
                | DecodeBuild::CompPerform { operation } => {
                    let payload = pop_decoded(&mut completed).into_value();
                    let signature = pop_decoded(&mut completed).into_effect_sig();
                    completed.push(Decoded::Comp(Comp::Perform(
                        Box::new(signature),
                        operation.to_owned(),
                        Rc::new(payload),
                    )));
                },
                | DecodeBuild::CompHandle {
                    return_binder,
                    operation_count,
                } => {
                    let mut operations = Vec::with_capacity(operation_count);
                    for _ in 0 .. operation_count {
                        operations.push(pop_decoded(&mut completed).into_op_clause());
                    }
                    operations.reverse();
                    let return_body = pop_decoded(&mut completed).into_comp();
                    let scrutinee = pop_decoded(&mut completed).into_comp();
                    let signature = pop_decoded(&mut completed).into_effect_sig();
                    completed.push(Decoded::Comp(Comp::Handle {
                        sig: Box::new(signature),
                        scrutinee: Rc::new(scrutinee),
                        ret: (return_binder.to_owned(), Rc::new(return_body)),
                        ops: operations,
                    }));
                },
                | DecodeBuild::CompResume => {
                    let body = pop_decoded(&mut completed).into_comp();
                    let value = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Comp(Comp::Resume(Rc::new(value), Rc::new(body))));
                },
                | DecodeBuild::CompReset => {
                    let body = pop_decoded(&mut completed).into_comp();
                    completed.push(Decoded::Comp(Comp::Reset(Rc::new(body))));
                },
                | DecodeBuild::CompShift { resume_binder } => {
                    let body = pop_decoded(&mut completed).into_comp();
                    completed.push(Decoded::Comp(Comp::Shift(
                        resume_binder.to_owned(),
                        Rc::new(body),
                    )));
                },
                | DecodeBuild::CompNative {
                    primitive,
                    argument_count,
                } => {
                    let mut arguments = Vec::with_capacity(argument_count);
                    for _ in 0 .. argument_count {
                        arguments.push(Rc::new(pop_decoded(&mut completed).into_value()));
                    }
                    arguments.reverse();
                    completed.push(Decoded::Comp(Comp::Native {
                        prim: primitive,
                        args: arguments,
                    }));
                },
                | DecodeBuild::CompWalk => {
                    let base = pop_decoded(&mut completed).into_walk_base();
                    let motive = pop_decoded(&mut completed).into_walk_motive();
                    let scrutinee = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Comp(Comp::Walk {
                        scrut: Rc::new(scrutinee),
                        motive: Box::new(motive),
                        base,
                    }));
                },
                | DecodeBuild::TermValue => {
                    let value = pop_decoded(&mut completed).into_value();
                    completed.push(Decoded::Term(Term::Value(value)));
                },
                | DecodeBuild::TermComp => {
                    let comp = pop_decoded(&mut completed).into_comp();
                    completed.push(Decoded::Term(Term::Comp(comp)));
                },
            },
        }
    }
    let result = pop_decoded(&mut completed).into_term();
    assert!(completed.is_empty(), "decoder left unattached child values");
    result
}

#[cfg(test)]
mod fixture_reader_contracts
{
    use super::Decoded;
    use super::Sexp;
    use super::Value;
    use super::parse_all;
    use super::pop_decoded;

    /// Parsing preserves both nested child order and top-level form order.
    #[test]
    fn parse_all_preserves_nested_and_top_level_order()
    {
        let parsed = parse_all("(first (second third)) fourth");

        assert_eq!(parsed, [
            Sexp::List(vec![
                Sexp::Sym("first".to_owned()),
                Sexp::List(vec![
                    Sexp::Sym("second".to_owned()),
                    Sexp::Sym("third".to_owned()),
                ]),
            ]),
            Sexp::Sym("fourth".to_owned()),
        ],);
    }

    /// Completed-child projection consumes the newest child and preserves prior
    /// children.
    #[test]
    fn pop_decoded_returns_latest_child()
    {
        let mut completed = vec![
            Decoded::Value(Value::Var("first".to_owned())),
            Decoded::Value(Value::Var("second".to_owned())),
        ];

        let newest = pop_decoded(&mut completed).into_value();
        assert_eq!(newest, Value::Var("second".to_owned()));
        assert_eq!(completed.len(), 1usize);

        let earlier = pop_decoded(&mut completed).into_value();
        assert_eq!(earlier, Value::Var("first".to_owned()));
        assert!(completed.is_empty());
    }
}
