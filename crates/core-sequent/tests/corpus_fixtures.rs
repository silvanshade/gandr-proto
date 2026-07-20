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
        reason = "a recursive-descent reader over trusted, self-generated \
                  fixtures: the positional slice indexing, the tag-shape \
                  assertions, and the by-ref enum-match binding modes are \
                  intrinsic to a compact parser (docs/workflow/rust.md)"
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
    let items = parse_all(text).iter().map(de_term).collect();
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
fn parse_all(input: &str) -> Vec<Sexp>
{
    let toks = tokenize(input);
    let mut pos = 0;
    let mut forms = Vec::new();
    while pos < toks.len() {
        forms.push(read_form(&toks, &mut pos));
    }
    forms
}

/// Reads one form from the token stream at `pos`, recursing into lists.
fn read_form(
    toks: &[Sexp],
    pos: &mut usize,
) -> Sexp
{
    let head = toks[*pos].clone();
    *pos += 1;
    match head {
        | Sexp::Sym(ref s) if s == "(" => {
            let mut items = Vec::new();
            loop {
                match &toks[*pos] {
                    | Sexp::Sym(s) if s == ")" => {
                        *pos += 1;
                        break;
                    },
                    | _ => items.push(read_form(toks, pos)),
                }
            }
            Sexp::List(items)
        },
        | Sexp::Sym(s) => Sexp::Sym(s),
        | Sexp::Str(s) => Sexp::Str(s),
        | Sexp::List(_) => panic!("tokenizer emits no lists"),
    }
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

/// Rebuilds an effect operation `(op "name" payload reply)`.
fn de_effop(s: &Sexp) -> EffectOp
{
    let v = as_list(s);
    assert_eq!(as_sym(&v[0]), "op");
    EffectOp::new(as_str(&v[1]).into(), de_vtype(&v[2]), de_vtype(&v[3]))
}

/// Rebuilds an effect signature `(sig "name" op...)`.
fn de_effsig(s: &Sexp) -> EffectSig
{
    let v = as_list(s);
    assert_eq!(as_sym(&v[0]), "sig");
    let ops = v[2 ..].iter().map(de_effop).collect();
    EffectSig::new(as_str(&v[1]).into(), ops)
}

/// Rebuilds an effect row `(row sig...)` by unioning singletons over `EMPTY`.
fn de_effrow(s: &Sexp) -> EffectRow
{
    let v = as_list(s);
    assert_eq!(as_sym(&v[0]), "row");
    v[1 ..].iter().fold(EffectRow::EMPTY, |acc, sig| {
        acc.union(&EffectRow::singleton(de_effsig(sig)))
    })
}

/// Rebuilds a value type.
fn de_vtype(s: &Sexp) -> ValueType
{
    match s {
        | Sexp::Sym(t) => match t.as_str() {
            | "tunit" => ValueType::Unit,
            | "tuniverse" => ValueType::Universe,
            | "tunknown" => ValueType::Unknown,
            | other => panic!("bad vtype sym {other}"),
        },
        | Sexp::List(v) => match as_sym(&v[0]) {
            | "tatom" => ValueType::Atom(as_str(&v[1]).to_owned()),
            | "tprod" => ValueType::Prod(Rc::new(de_vtype(&v[1])), Rc::new(de_vtype(&v[2]))),
            | "tsum" => ValueType::Sum(Rc::new(de_vtype(&v[1])), Rc::new(de_vtype(&v[2]))),
            | "tlist" => ValueType::List(Rc::new(de_vtype(&v[1]))),
            | "trec" => {
                let mut m = BTreeMap::new();
                for field in &v[1 ..] {
                    let f = as_list(field);
                    m.insert(as_str(&f[0]).to_owned(), Rc::new(de_vtype(&f[1])));
                }
                ValueType::Record(m)
            },
            | "tthunk" => ValueType::Thunk(de_grade(&v[1]), Rc::new(de_ctype(&v[2]))),
            | "tstk" => ValueType::Stk(Rc::new(de_ctype(&v[1])), Rc::new(de_ctype(&v[2]))),
            | "tpath" => ValueType::path(de_vtype(&v[1]), de_value(&v[2]), de_value(&v[3])),
            | "tdata" => {
                let id = de_dataid(&v[1]);
                let args = v[2 ..].iter().map(de_vtype).collect();
                ValueType::data(id, args)
            },
            | other => panic!("bad vtype list {other}"),
        },
        | other => panic!("bad vtype {other:?}"),
    }
}

/// Rebuilds a computation type.
fn de_ctype(s: &Sexp) -> CompType
{
    match s {
        | Sexp::Sym(t) if t == "ctunknown" => CompType::Unknown,
        | Sexp::List(v) => match as_sym(&v[0]) {
            | "ctf" => CompType::F(Rc::new(de_vtype(&v[1])), de_effrow(&v[2])),
            | "ctarrow" => CompType::Arrow(Rc::new(de_vtype(&v[1])), Rc::new(de_ctype(&v[2]))),
            | "ctwith" => CompType::With(Rc::new(de_ctype(&v[1])), Rc::new(de_ctype(&v[2]))),
            | other => panic!("bad ctype list {other}"),
        },
        | other => panic!("bad ctype {other:?}"),
    }
}

/// Rebuilds an optional binder annotation `none` / `(some vtype)`.
fn de_opt_vtype(s: &Sexp) -> Option<Rc<ValueType>>
{
    match s {
        | Sexp::Sym(t) if t == "none" => None,
        | Sexp::List(v) => {
            assert_eq!(as_sym(&v[0]), "some");
            Some(Rc::new(de_vtype(&v[1])))
        },
        | other => panic!("bad opt vtype {other:?}"),
    }
}

/// Rebuilds an optional split motive `none` / `(some "binder" ctype)`.
fn de_splitmotive(s: &Sexp) -> Option<Box<SplitMotive>>
{
    match s {
        | Sexp::Sym(t) if t == "none" => None,
        | Sexp::List(v) => {
            assert_eq!(as_sym(&v[0]), "some");
            Some(Box::new(SplitMotive::new(as_str(&v[1]), de_ctype(&v[2]))))
        },
        | other => panic!("bad split motive {other:?}"),
    }
}

/// Rebuilds a Walk motive `(wmotive "x" "y" "q" ctype)`.
fn de_walkmotive(s: &Sexp) -> WalkMotive
{
    let v = as_list(s);
    assert_eq!(as_sym(&v[0]), "wmotive");
    WalkMotive::new(as_str(&v[1]), as_str(&v[2]), as_str(&v[3]), de_ctype(&v[4]))
}

/// Rebuilds a Walk base `(wbase "x" comp)`.
fn de_walkbase(s: &Sexp) -> WalkBase
{
    let v = as_list(s);
    assert_eq!(as_sym(&v[0]), "wbase");
    WalkBase::new(as_str(&v[1]), de_comp(&v[2]))
}

/// Rebuilds a handler operation clause `(opclause "op" "p" "k" body)`.
fn de_opclause(s: &Sexp) -> OpClause
{
    let v = as_list(s);
    assert_eq!(as_sym(&v[0]), "opclause");
    OpClause::new(as_str(&v[1]), as_str(&v[2]), as_str(&v[3]), de_comp(&v[4]))
}

/// Rebuilds a value.
fn de_value(s: &Sexp) -> Value
{
    match s {
        | Sexp::Sym(t) if t == "u" => Value::Unit,
        | Sexp::List(v) => match as_sym(&v[0]) {
            | "var" => Value::Var(as_str(&v[1]).to_owned()),
            | "i" => Value::Int(as_sym(&v[1]).parse().unwrap()),
            | "s" => Value::Str(as_str(&v[1]).to_owned()),
            | "n" => Value::Num(de_numlit(&v[1])),
            | "pair" => Value::Pair(Rc::new(de_value(&v[1])), Rc::new(de_value(&v[2]))),
            | "inj" => Value::Inj(de_side(&v[1]), Rc::new(de_value(&v[2]))),
            | "vlist" => Value::List(v[1 ..].iter().map(|e| Rc::new(de_value(e))).collect()),
            | "vrec" => {
                let mut m = BTreeMap::new();
                for field in &v[1 ..] {
                    let f = as_list(field);
                    m.insert(as_str(&f[0]).to_owned(), Rc::new(de_value(&f[1])));
                }
                Value::Record(m)
            },
            | "thunk" => Value::Thunk(de_grade(&v[1]), Rc::new(de_comp(&v[2]))),
            | "annot" => Value::Annot(Rc::new(de_value(&v[1])), Rc::new(de_vtype(&v[2]))),
            | "vhole" => Value::Hole(as_sym(&v[1]).parse().unwrap()),
            | "here" => Value::Here(Rc::new(de_value(&v[1]))),
            | "ctor" => Value::Ctor {
                id: de_dataid(&v[1]),
                tag: as_sym(&v[2]).parse().unwrap(),
                payload: Rc::new(de_value(&v[3])),
            },
            | other => panic!("bad value list {other}"),
        },
        | other => panic!("bad value {other:?}"),
    }
}

/// Rebuilds a computation.
fn de_comp(s: &Sexp) -> Comp
{
    let v = as_list(s);
    match as_sym(&v[0]) {
        | "abs" => Comp::Abs(
            as_str(&v[1]).to_owned(),
            de_opt_vtype(&v[2]),
            Rc::new(de_comp(&v[3])),
        ),
        | "app" => Comp::App(Rc::new(de_comp(&v[1])), Rc::new(de_value(&v[2]))),
        | "ret" => Comp::Ret(Rc::new(de_value(&v[1]))),
        | "bind" => Comp::Bind(
            Rc::new(de_comp(&v[1])),
            as_str(&v[2]).to_owned(),
            Rc::new(de_comp(&v[3])),
        ),
        | "force" => Comp::Force(Rc::new(de_value(&v[1]))),
        | "case" => Comp::Case(
            Rc::new(de_value(&v[1])),
            (as_str(&v[2]).to_owned(), Rc::new(de_comp(&v[3]))),
            (as_str(&v[4]).to_owned(), Rc::new(de_comp(&v[5]))),
        ),
        | "datacase" => {
            let scrut = Rc::new(de_value(&v[1]));
            let arms = v[2 ..]
                .iter()
                .map(|arm| {
                    let a = as_list(arm);
                    assert_eq!(as_sym(&a[0]), "arm");
                    (as_str(&a[1]).to_owned(), Rc::new(de_comp(&a[2])))
                })
                .collect();
            Comp::DataCase(scrut, arms)
        },
        | "listcase" => Comp::ListCase {
            scrut: Rc::new(de_value(&v[1])),
            nil: Rc::new(de_comp(&v[2])),
            head: as_str(&v[3]).to_owned(),
            tail: as_str(&v[4]).to_owned(),
            cons: Rc::new(de_comp(&v[5])),
        },
        | "split" => Comp::Split {
            scrut: Rc::new(de_value(&v[1])),
            fst_name: as_str(&v[2]).to_owned(),
            snd_name: as_str(&v[3]).to_owned(),
            motive: de_splitmotive(&v[4]),
            body: Rc::new(de_comp(&v[5])),
        },
        | "recordproj" => Comp::RecordProj {
            record: Rc::new(de_value(&v[1])),
            label: as_str(&v[2]).to_owned(),
        },
        | "cwith" => Comp::With(Rc::new(de_comp(&v[1])), Rc::new(de_comp(&v[2]))),
        | "prj" => Comp::Prj(de_side(&v[1]), Rc::new(de_comp(&v[2]))),
        | "dup" => Comp::Dup(Rc::new(de_value(&v[1]))),
        | "drop" => Comp::Drop(Rc::new(de_value(&v[1]))),
        | "perform" => Comp::Perform(
            Box::new(de_effsig(&v[1])),
            as_str(&v[2]).to_owned(),
            Rc::new(de_value(&v[3])),
        ),
        | "handle" => {
            let ops = v[5 ..].iter().map(de_opclause).collect();
            Comp::Handle {
                sig: Box::new(de_effsig(&v[1])),
                scrutinee: Rc::new(de_comp(&v[2])),
                ret: (as_str(&v[3]).to_owned(), Rc::new(de_comp(&v[4]))),
                ops,
            }
        },
        | "resume" => Comp::Resume(Rc::new(de_value(&v[1])), Rc::new(de_comp(&v[2]))),
        | "reset" => Comp::Reset(Rc::new(de_comp(&v[1]))),
        | "shift" => Comp::Shift(as_str(&v[1]).to_owned(), Rc::new(de_comp(&v[2]))),
        | "chole" => Comp::Hole(as_sym(&v[1]).parse().unwrap()),
        | "native" => Comp::Native {
            prim: de_prim(&v[1]),
            args: v[2 ..].iter().map(|a| Rc::new(de_value(a))).collect(),
        },
        | "walk" => Comp::Walk {
            scrut: Rc::new(de_value(&v[1])),
            motive: Box::new(de_walkmotive(&v[2])),
            base: de_walkbase(&v[3]),
        },
        | other => panic!("bad comp {other}"),
    }
}

/// Rebuilds a top-level term `(v value)` / `(c comp)`.
fn de_term(s: &Sexp) -> Term
{
    let v = as_list(s);
    match as_sym(&v[0]) {
        | "v" => Term::Value(de_value(&v[1])),
        | "c" => Term::Comp(de_comp(&v[1])),
        | other => panic!("bad term {other}"),
    }
}
