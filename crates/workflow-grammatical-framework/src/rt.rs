//! The `GfRuntime` trait (the internalization seam) and its `PyO3` backend.
//!
//! Everything outside this module talks to the trait only; the `PyO3` backend
//! drives the `pgf` Python binding inside an embedded interpreter whose
//! environment is the uv project declared in this crate's `pyproject.toml`.
//!
//! The backend models the binding's object graph as nominal `Rust` wrappers
//! (the vkpp wrapper pattern): each wrapper holds its live Python handle
//! privately and exposes methods mirroring the `Python` `API` one-for-one,
//! forwarding to the inner object. Validation lives at the naturally
//! fallible boundaries (module import, `.pgf` load, language selection);
//! returned objects are trusted — a bad access surfaces as a typed error at
//! the forwarded call, never a crash (`PyO3`'s guarantee), so no defensive
//! shape-checking duplicates the `API`'s own contract.

use alloc::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use pyo3::exceptions::PyException;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyList;
use pyo3::types::PyModule;
use pyo3::types::PyTuple;

use crate::error::GfError;
use crate::sexp::Sexp;

/// A `GF` expression in its textual surface form: the input of the reading,
/// checking, and linearization lanes.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct ExprText
{
    /// The expression source text.
    text: String,
}

impl ExprText
{
    /// Wrap expression source text.
    #[inline]
    #[must_use]
    pub fn new<T>(text: T) -> Self
    where
        T: Into<String>,
    {
        Self { text: text.into() }
    }
}

impl AsRef<str> for ExprText
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.text
    }
}

impl core::fmt::Display for ExprText
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        self.text.fmt(f)
    }
}

/// A category name in the loaded grammar (`Term`, `Inline`, …).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct CategoryName
{
    /// The category identifier.
    name: String,
}

impl CategoryName
{
    /// Wrap a category identifier.
    #[inline]
    #[must_use]
    pub fn new<T>(name: T) -> Self
    where
        T: Into<String>,
    {
        Self { name: name.into() }
    }
}

impl AsRef<str> for CategoryName
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.name
    }
}

impl core::fmt::Display for CategoryName
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        self.name.fmt(f)
    }
}

/// A function (constructor) name in the loaded grammar.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct FunctionName
{
    /// The function identifier.
    name: String,
}

impl FunctionName
{
    /// Wrap a function identifier.
    #[inline]
    #[must_use]
    pub fn new<T>(name: T) -> Self
    where
        T: Into<String>,
    {
        Self { name: name.into() }
    }
}

impl AsRef<str> for FunctionName
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.name
    }
}

impl core::fmt::Display for FunctionName
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        self.name.fmt(f)
    }
}

/// A concrete-syntax (language) name in the loaded grammar.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct LanguageName
{
    /// The language identifier.
    name: String,
}

impl LanguageName
{
    /// Wrap a language identifier.
    #[inline]
    #[must_use]
    pub fn new<T>(name: T) -> Self
    where
        T: Into<String>,
    {
        Self { name: name.into() }
    }
}

impl AsRef<str> for LanguageName
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.name
    }
}

impl core::fmt::Display for LanguageName
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        self.name.fmt(f)
    }
}

/// A single word form for morphological lookup.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct WordText
{
    /// The word form.
    text: String,
}

impl WordText
{
    /// Wrap a word form.
    #[inline]
    #[must_use]
    pub fn new<T>(text: T) -> Self
    where
        T: Into<String>,
    {
        Self { text: text.into() }
    }
}

impl AsRef<str> for WordText
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.text
    }
}

/// A sentence (or sentence fragment) of authored prose for the parse lane.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct SentenceText
{
    /// The sentence text.
    text: String,
}

impl SentenceText
{
    /// Wrap sentence text.
    #[inline]
    #[must_use]
    pub fn new<T>(text: T) -> Self
    where
        T: Into<String>,
    {
        Self { text: text.into() }
    }
}

impl AsRef<str> for SentenceText
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.text
    }
}

/// One morphological analysis of a word form (the binding's `lookupMorpho`
/// result: the lexeme and category, e.g. `completion_N`).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct MorphoAnalysis
{
    /// The analysis string.
    analysis: String,
}

impl MorphoAnalysis
{
    /// Wrap an analysis string.
    #[inline]
    #[must_use]
    pub fn new<T>(analysis: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            analysis: analysis.into(),
        }
    }
}

impl AsRef<str> for MorphoAnalysis
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.analysis
    }
}

/// One parse result: the runtime's probability rank and the parsed tree.
#[derive(Debug)]
#[non_exhaustive]
pub struct ScoredTree
{
    /// The parse's probability score (higher is better-ranked).
    pub probability: f64,
    /// The parsed expression.
    pub tree: PgfExpr,
}

/// The parse-result cap (bounded because the runtime's parse is lazy and
/// unbounded in principle).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct ParseLimit
{
    /// The cap (clamped to 32 at construction).
    limit: usize,
}

impl ParseLimit
{
    /// The largest cap the lane accepts.
    pub const MAX: usize = 32;

    /// Wrap a cap, clamped to [`ParseLimit::MAX`].
    #[inline]
    #[must_use]
    pub fn new(limit: usize) -> Self
    {
        Self {
            limit: limit.min(Self::MAX),
        }
    }

    /// The clamped value.
    #[inline]
    #[must_use]
    pub fn get(&self) -> usize
    {
        self.limit
    }
}

/// The record-field linearization of one expression (the runtime's
/// `tabularLinearize` result): field paths mapped to their linearized text.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct TabularLinearization
{
    /// The field-path map.
    fields: BTreeMap<String, String>,
}

impl TabularLinearization
{
    /// Wrap extracted record fields.
    pub(crate) fn new(fields: BTreeMap<String, String>) -> Self
    {
        Self { fields }
    }
}

impl core::ops::Deref for TabularLinearization
{
    type Target = BTreeMap<String, String>;

    #[inline]
    fn deref(&self) -> &Self::Target
    {
        &self.fields
    }
}

/// The faithful deconstruction of one `pgf.Expr` (the binding's `unpack`):
/// either a string literal or a constructor application.
#[derive(Debug)]
#[non_exhaustive]
pub enum ExprUnpack
{
    /// A string literal.
    Literal(String),
    /// A constructor application: the head and its arguments (empty for a
    /// nullary constructor).
    Application
    {
        /// The constructor name.
        head: String,
        /// The argument expressions.
        args: Vec<PgfExpr>,
    },
}

/// The imported `pgf` module: the binding's module-level entry points
/// (`readPGF`, `readExpr`) as methods.
#[repr(transparent)]
pub struct PgfModule
{
    /// The live module handle.
    handle: Py<PyModule>,
}

impl PgfModule
{
    /// Import the `pgf` module.
    ///
    /// The crate's uv environment (`.venv` beside this manifest) is prepended
    /// to `sys.path` when present, so the binary needs no `PYTHONPATH`.
    ///
    /// # Errors
    /// [`GfError::Python`] when the interpreter or the import fails (the
    /// diagnostic probes the interpreter's executable, version, and path).
    #[inline]
    pub fn import() -> Result<Self, GfError>
    {
        Python::attach(|py| {
            prepend_venv_site_packages(py)?;
            let module = PyModule::import(py, "pgf").map_err(|e| import_err(py, &e))?;
            Ok(Self {
                handle: module.unbind(),
            })
        })
    }

    /// `pgf.readPGF(path)`: load a compiled grammar image.
    ///
    /// # Errors
    /// [`GfError::Python`] when the load fails (missing or invalid `.pgf`).
    #[inline]
    pub fn read_pgf(
        &self,
        path: &Path,
    ) -> Result<PgfGrammar, GfError>
    {
        Python::attach(|py| {
            let grammar = self
                .handle
                .bind(py)
                .getattr("readPGF")
                .and_then(|f| f.call1((path.to_string_lossy().as_ref(),)))
                .map_err(|e| py_err(&e))?;
            Ok(PgfGrammar::wrap(grammar.unbind()))
        })
    }

    /// `pgf.readExpr(text)`: read one expression (untyped — validation is
    /// the pipeline's explicit `checkExpr` lane).
    ///
    /// # Errors
    /// [`GfError::Python`] when the expression text is unreadable.
    #[inline]
    pub fn read_expr(
        &self,
        text: &ExprText,
    ) -> Result<PgfExpr, GfError>
    {
        Python::attach(|py| {
            let expr = self
                .handle
                .bind(py)
                .getattr("readExpr")
                .and_then(|f| f.call1((text.as_ref(),)))
                .map_err(|e| py_err(&e))?;
            Ok(PgfExpr::wrap(expr.unbind()))
        })
    }

    /// `pgf.readType(name)`: read one type expression (a start category for
    /// the parse lane).
    ///
    /// # Errors
    /// [`GfError::Python`] when the type text is unreadable.
    #[inline]
    pub fn read_type(
        &self,
        cat: &CategoryName,
    ) -> Result<PgfType, GfError>
    {
        Python::attach(|py| {
            let cat_type = self
                .handle
                .bind(py)
                .getattr("readType")
                .and_then(|f| f.call1((cat.as_ref(),)))
                .map_err(|e| py_err(&e))?;
            Ok(PgfType::wrap(cat_type.unbind()))
        })
    }
}

/// A loaded grammar image (the binding's `pgf.PGF` object): language
/// selection and the grammar-introspection and checking methods.
#[repr(transparent)]
pub struct PgfGrammar
{
    /// The live `pgf.PGF` handle.
    handle: Py<PyAny>,
}

impl PgfGrammar
{
    /// Wrap a live `pgf.PGF` handle (crate-private: the construction sites
    /// are the binding's own `readPGF`; the `API` contract carries the type,
    /// and a bad access would surface as a typed error at the first
    /// forwarded call).
    fn wrap(handle: Py<PyAny>) -> Self
    {
        Self { handle }
    }

    /// `gr.languages[name]`: select a concrete syntax by name.
    ///
    /// # Errors
    /// [`GfError::Python`] when the grammar has no such concrete syntax.
    #[inline]
    pub fn language(
        &self,
        name: &LanguageName,
    ) -> Result<PgfConcrete, GfError>
    {
        Python::attach(|py| {
            let languages = self
                .handle
                .bind(py)
                .getattr("languages")
                .map_err(|e| py_err(&e))?;
            let concrete = languages.get_item(name.as_ref()).map_err(|e| py_err(&e))?;
            Ok(PgfConcrete::wrap(concrete.unbind()))
        })
    }

    /// `gr.functionsByCat(cat)`: the category's function names.
    ///
    /// # Errors
    /// [`GfError::Python`] on interop failure (an unknown category
    /// included).
    #[inline]
    pub fn functions_by_cat(
        &self,
        cat: &CategoryName,
    ) -> Result<Vec<FunctionName>, GfError>
    {
        Python::attach(|py| {
            let names: Vec<String> = self
                .handle
                .bind(py)
                .getattr("functionsByCat")
                .and_then(|f| f.call1((cat.as_ref(),)))
                .and_then(|r| r.extract())
                .map_err(|e| py_err(&e))?;
            Ok(names.into_iter().map(FunctionName::new).collect())
        })
    }

    /// `gr.inferExpr(expr)`: validate an expression at the `checkExpr` lane
    /// (and solve its metavariables), returning the checked expression.
    ///
    /// # Errors
    /// [`GfError::Pgf`] when the runtime rejects the tree (unknown function
    /// or ill-typed application); [`GfError::Python`] on interop failure.
    #[inline]
    pub fn infer_expr(
        &self,
        expr: &PgfExpr,
    ) -> Result<PgfExpr, GfError>
    {
        Python::attach(|py| {
            let result = self
                .handle
                .bind(py)
                .getattr("inferExpr")
                .and_then(|f| f.call1((&expr.handle,)))
                .map_err(|e| check_err(&e))?;
            let pair = result
                .cast::<PyTuple>()
                .map_err(|e| GfError::Python(format!("inferExpr did not return a pair: {e:?}")))?;
            let checked = pair.get_item(0).map_err(|e| py_err(&e))?;
            Ok(PgfExpr::wrap(checked.unbind()))
        })
    }
}

/// One concrete syntax (the binding's `pgf.Concr` object): the
/// linearization methods.
#[repr(transparent)]
pub struct PgfConcrete
{
    /// The live `pgf.Concr` handle.
    handle: Py<PyAny>,
}

impl PgfConcrete
{
    /// Wrap a live `pgf.Concr` handle (crate-private; the `API` contract of
    /// `languages[…]` carries the type).
    fn wrap(handle: Py<PyAny>) -> Self
    {
        Self { handle }
    }

    /// `concr.linearize(expr)`: render the expression in this concrete
    /// syntax.
    ///
    /// # Errors
    /// [`GfError::Python`] on interop failure.
    #[inline]
    pub fn linearize(
        &self,
        expr: &PgfExpr,
    ) -> Result<String, GfError>
    {
        Python::attach(|py| {
            self.handle
                .bind(py)
                .getattr("linearize")
                .and_then(|f| f.call1((&expr.handle,)))
                .and_then(|s| s.extract::<String>())
                .map_err(|e| py_err(&e))
        })
    }

    /// `concr.tabularLinearize(expr)`: the record-field linearization of
    /// one expression (field paths mapped to their text).
    ///
    /// # Errors
    /// [`GfError::Python`] on interop failure.
    #[inline]
    pub fn tabular_linearize(
        &self,
        expr: &PgfExpr,
    ) -> Result<TabularLinearization, GfError>
    {
        Python::attach(|py| {
            let fields: BTreeMap<String, String> = self
                .handle
                .bind(py)
                .getattr("tabularLinearize")
                .and_then(|f| f.call1((&expr.handle,)))
                .and_then(|r| r.extract())
                .map_err(|e| py_err(&e))?;
            Ok(TabularLinearization::new(fields))
        })
    }

    /// `concr.lookupMorpho(word)`: the morphological analyses of one word
    /// form (empty when the word is unknown to the lexicon).
    ///
    /// # Errors
    /// [`GfError::Python`] on interop failure.
    #[inline]
    pub fn lookup_morpho(
        &self,
        word: &WordText,
    ) -> Result<Vec<MorphoAnalysis>, GfError>
    {
        Python::attach(|py| {
            let analyses: Vec<String> = self
                .handle
                .bind(py)
                .getattr("lookupMorpho")
                .and_then(|f| f.call1((word.as_ref(),)))
                .and_then(|r| r.extract())
                .map_err(|e| py_err(&e))?;
            Ok(analyses.into_iter().map(MorphoAnalysis::new).collect())
        })
    }

    /// `concr.parse(sentence, cat=…)`: parse prose, returning the
    /// best-ranked results (lazily, probability-ordered) up to the cap.
    ///
    /// The caller guards the input (sentence length and complexity) — the C
    /// parser can exhaust its stack on long ambiguous inputs (the gandr-aaq
    /// spike's debugger-verified crash, `internalizing-gf.md`).
    ///
    /// # Errors
    /// [`GfError::Pgf`] when the sentence does not parse (the runtime's
    /// `ParseError`); [`GfError::Python`] on other interop failure.
    #[inline]
    pub fn parse(
        &self,
        sentence: &SentenceText,
        cat: &PgfType,
        limit: ParseLimit,
    ) -> Result<Vec<ScoredTree>, GfError>
    {
        Python::attach(|py| {
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs
                .set_item("cat", &cat.handle)
                .map_err(|e| py_err(&e))?;
            let iterator = self
                .handle
                .bind(py)
                .getattr("parse")
                .and_then(|f| f.call((sentence.as_ref(),), Some(&kwargs)))
                .map_err(|e| parse_err(py, &e))?;
            let mut results = Vec::new();
            let iter = iterator.try_iter().map_err(|e| py_err(&e))?;
            for item in iter.take(limit.get()) {
                let item = item.map_err(|e| parse_err(py, &e))?;
                let (probability, tree): (f64, Bound<'_, PyAny>) =
                    item.extract().map_err(|e| py_err(&e))?;
                results.push(ScoredTree {
                    probability,
                    tree: PgfExpr::wrap(tree.unbind()),
                });
            }
            Ok(results)
        })
    }
}

/// One expression (the binding's `pgf.Expr` object): tree deconstruction.
#[derive(Debug)]
#[repr(transparent)]
pub struct PgfExpr
{
    /// The live `pgf.Expr` handle.
    handle: Py<PyAny>,
}

/// One type expression (the binding's `pgf.Type` object): the parse lane's
/// start categories.
#[repr(transparent)]
pub struct PgfType
{
    /// The live `pgf.Type` handle.
    handle: Py<PyAny>,
}

impl PgfType
{
    /// Wrap a live `pgf.Type` handle (crate-private; the `API` contract of
    /// `readType` carries the type).
    fn wrap(handle: Py<PyAny>) -> Self
    {
        Self { handle }
    }
}

impl PgfExpr
{
    /// Wrap a live `pgf.Expr` handle (crate-private; the `API` contract of
    /// `readExpr`/`inferExpr` carries the type).
    fn wrap(handle: Py<PyAny>) -> Self
    {
        Self { handle }
    }

    /// `expr.unpack()`: deconstruct the expression — a string literal, or
    /// an application (head, arguments).
    ///
    /// # Errors
    /// [`GfError::Python`] when the deconstruction returns neither a string
    /// nor an application pair.
    #[inline]
    pub fn unpack(&self) -> Result<ExprUnpack, GfError>
    {
        Python::attach(|py| {
            let unpacked = self
                .handle
                .bind(py)
                .call_method0("unpack")
                .map_err(|e| py_err(&e))?;
            if let Ok(text) = unpacked.extract::<String>() {
                return Ok(ExprUnpack::Literal(text));
            }
            let tuple = unpacked.cast::<PyTuple>().map_err(|error| {
                GfError::Python(format!(
                    "pgf.Expr.unpack returned neither str nor tuple: {error:?}"
                ))
            })?;
            let head = tuple
                .get_item(0)
                .and_then(|item| item.extract::<String>())
                .map_err(|e| py_err(&e))?;
            let args_obj = tuple.get_item(1).map_err(|e| py_err(&e))?;
            let list = args_obj.cast::<PyList>().map_err(|error| {
                GfError::Python(format!(
                    "pgf.Expr.unpack arguments are not a list: {error:?}"
                ))
            })?;
            let mut args = Vec::with_capacity(list.len());
            for item in list.iter() {
                args.push(Self::wrap(item.unbind()));
            }
            Ok(ExprUnpack::Application { head, args })
        })
    }
}

/// Convert one `pgf.Expr` into the crate tree: string literals become
/// string atoms (re-quoted), nullary constructors bare atoms, and
/// applications constructor applications — the historical reader's shapes,
/// so every tree consumer sees one representation.
///
/// The conversion runs an explicit frame stack (the house traversal pattern):
/// each `Enter` frame decomposes one node, each `Complete` frame folds its
/// already-converted children. Arguments push in reverse so they convert in
/// source order (depth-first, each subtree completed before the next starts).
///
/// # Termination
/// - reason: an explicit work stack decomposes each expression node into strict
///   children, visited once.
/// - measure: pending frames; every `Complete` pops after exactly its
///   children's conversions.
/// - boundedness: the source expression is a finite tree.
/// - input recursion: none.
fn sexp_of(root: PgfExpr) -> Result<Sexp, GfError>
{
    /// One work-stack frame: decompose a node, record a literal, or fold
    /// converted children.
    enum Frame
    {
        /// Decompose this expression.
        Enter(PgfExpr),
        /// Record a string literal (already deconstructed).
        Literal(String),
        /// Fold the next `arity` converted trees under `head`.
        Complete
        {
            /// The constructor name.
            head: String,
            /// How many converted children to fold.
            arity: usize,
        },
    }

    let mut done: Vec<Sexp> = Vec::new();
    let mut stack = vec![Frame::Enter(root)];
    while let Some(frame) = stack.pop() {
        match frame {
            | Frame::Enter(node) => match node.unpack()? {
                | ExprUnpack::Literal(text) => stack.push(Frame::Literal(text)),
                | ExprUnpack::Application { head, args } => {
                    stack.push(Frame::Complete {
                        head,
                        arity: args.len(),
                    });
                    for arg in args.into_iter().rev() {
                        stack.push(Frame::Enter(arg));
                    }
                },
            },
            | Frame::Literal(text) => done.push(Sexp::string(text)),
            | Frame::Complete { head, arity } => {
                if arity == 0 {
                    done.push(Sexp::atom(head));
                    continue;
                }
                let children = done.split_off(done.len().saturating_sub(arity));
                done.push(Sexp::app(head, children));
            },
        }
    }
    done.pop()
        .ok_or_else(|| GfError::Python("expression conversion produced no tree".to_owned()))
}

/// The runtime surface the pipeline uses (proposal §4).
pub trait GfRuntime
{
    /// Read an expression (`.gfd` text), validate it at the `checkExpr` lane
    /// (`inferExpr`), and linearize it in the loaded concrete syntax.
    ///
    /// # Errors
    ///
    /// [`GfError::Pgf`] when the runtime rejects the tree (unknown function or
    /// ill-typed application); [`GfError::Python`] on interop failure.
    fn check_and_linearize(
        &self,
        expr: &ExprText,
    ) -> Result<String, GfError>;

    /// Validate without rendering (the negative-test lane).
    ///
    /// # Errors
    /// As [`GfRuntime::check_and_linearize`].
    fn check(
        &self,
        expr: &ExprText,
    ) -> Result<(), GfError>;

    /// Read an expression into the crate tree (the runtime's `readExpr` plus
    /// tree deconstruction at the boundary). Grammar-independent: the loaded
    /// grammar is not consulted.
    ///
    /// # Errors
    /// [`GfError::Python`] when the expression text is unreadable or the
    /// deconstruction fails.
    fn read_tree(
        &self,
        expr: &ExprText,
    ) -> Result<Sexp, GfError>;

    /// The grammar's function names in one category.
    ///
    /// # Errors
    /// [`GfError::Python`] on interop failure (an unknown category included).
    fn functions_by_cat(
        &self,
        cat: &CategoryName,
    ) -> Result<Vec<FunctionName>, GfError>;

    /// The record-field linearization of one expression in the loaded
    /// concrete syntax (the runtime's `tabularLinearize`).
    ///
    /// # Errors
    /// [`GfError::Python`] on interop failure.
    fn tabular_linearize(
        &self,
        expr: &ExprText,
    ) -> Result<TabularLinearization, GfError>;

    /// The morphological analyses of one word form in the loaded concrete
    /// syntax (empty when the lexicon does not know the word).
    ///
    /// # Errors
    /// [`GfError::Python`] on interop failure.
    fn lookup_morpho(
        &self,
        word: &WordText,
    ) -> Result<Vec<MorphoAnalysis>, GfError>;

    /// Parse prose in the loaded concrete syntax (bounded, best-ranked
    /// first). The caller guards sentence length and complexity.
    ///
    /// # Errors
    /// [`GfError::Pgf`] when the sentence does not parse;
    /// [`GfError::Python`] on other interop failure.
    fn parse(
        &self,
        sentence: &SentenceText,
        cat: &CategoryName,
        limit: ParseLimit,
    ) -> Result<Vec<ScoredTree>, GfError>;
}

/// The `PyO3` backend: the imported module, the loaded grammar, and the
/// selected concrete syntax, each behind its nominal wrapper.
pub struct PyPgf
{
    /// The imported `pgf` module (module-level `readExpr` lives here).
    module: PgfModule,
    /// The loaded grammar image.
    grammar: PgfGrammar,
    /// The concrete syntax selected for linearization.
    concrete: PgfConcrete,
}

impl PyPgf
{
    /// Load a compiled `.pgf` and select a concrete syntax by name.
    ///
    /// # Errors
    /// [`GfError::Python`] when the interpreter, the `pgf` import, the
    /// `.pgf` load, or the language lookup fails.
    #[inline]
    pub fn new(
        pgf_path: &Path,
        language: &LanguageName,
    ) -> Result<Self, GfError>
    {
        let module = PgfModule::import()?;
        let grammar = module.read_pgf(pgf_path)?;
        let concrete = grammar.language(language)?;
        Ok(Self {
            module,
            grammar,
            concrete,
        })
    }
}

impl GfRuntime for PyPgf
{
    #[inline]
    fn check_and_linearize(
        &self,
        expr: &ExprText,
    ) -> Result<String, GfError>
    {
        let tree = self.module.read_expr(expr)?;
        let checked = self.grammar.infer_expr(&tree)?;
        self.concrete.linearize(&checked)
    }

    #[inline]
    fn check(
        &self,
        expr: &ExprText,
    ) -> Result<(), GfError>
    {
        let tree = self.module.read_expr(expr)?;
        self.grammar.infer_expr(&tree).map(|_| ())
    }

    #[inline]
    fn read_tree(
        &self,
        expr: &ExprText,
    ) -> Result<Sexp, GfError>
    {
        let tree = self.module.read_expr(expr)?;
        sexp_of(tree)
    }

    #[inline]
    fn functions_by_cat(
        &self,
        cat: &CategoryName,
    ) -> Result<Vec<FunctionName>, GfError>
    {
        self.grammar.functions_by_cat(cat)
    }

    #[inline]
    fn tabular_linearize(
        &self,
        expr: &ExprText,
    ) -> Result<TabularLinearization, GfError>
    {
        let tree = self.module.read_expr(expr)?;
        self.concrete.tabular_linearize(&tree)
    }

    #[inline]
    fn lookup_morpho(
        &self,
        word: &WordText,
    ) -> Result<Vec<MorphoAnalysis>, GfError>
    {
        self.concrete.lookup_morpho(word)
    }

    #[inline]
    fn parse(
        &self,
        sentence: &SentenceText,
        cat: &CategoryName,
        limit: ParseLimit,
    ) -> Result<Vec<ScoredTree>, GfError>
    {
        let cat_type = self.module.read_type(cat)?;
        self.concrete.parse(sentence, &cat_type, limit)
    }
}

/// Map an interop-side failure (import, attribute, conversion) to the
/// interop variant.
fn py_err(error: &PyErr) -> GfError
{
    GfError::Python(error.to_string())
}

/// Map a module-import failure to the interop variant, probing the
/// interpreter's executable, version, and path for the diagnostic.
fn import_err(
    py: Python<'_>,
    error: &PyErr,
) -> GfError
{
    let probe = py
        .import("sys")
        .and_then(|sys| {
            let exe_attr = sys.getattr("executable")?;
            let exe = exe_attr.extract::<String>()?;
            let ver_attr = sys.getattr("version")?;
            let ver = ver_attr.extract::<String>()?;
            let path_attr = sys.getattr("path")?;
            let path = path_attr.extract::<Vec<String>>()?;
            Ok(format!(
                "executable={exe} version={} path={path:?}",
                ver.chars().take(6).collect::<String>()
            ))
        })
        .unwrap_or_else(|_| "probe failed".to_owned());
    GfError::Python(format!("{error} [{probe}]"))
}

/// Prepend the crate's uv `site-packages` to `sys.path` when the directory
/// exists (the `.venv` lives beside this crate's manifest at build time).
#[cfg_attr(
    dylint_lib = "abs_home_path",
    allow(
        abs_home_path,
        reason = "CARGO_MANIFEST_DIR is the build-time anchor for the crate's own uv \
                  environment; this tool only ever runs from the repository checkout"
    )
)]
fn prepend_venv_site_packages(py: Python<'_>) -> Result<(), GfError>
{
    let lib = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.venv/lib"));
    let Ok(mut versions) = std::fs::read_dir(&lib)
    else {
        return Ok(());
    };
    let Some(Ok(version)) = versions.next()
    else {
        return Ok(());
    };
    let site = version.path().join("site-packages");
    if !site.is_dir() {
        return Ok(());
    }
    let sys = py.import("sys").map_err(|e| py_err(&e))?;
    let path = sys.getattr("path").map_err(|e| py_err(&e))?;
    path.call_method1("insert", (0_i32, site.to_string_lossy().into_owned()))
        .map_err(|e| py_err(&e))?;
    Ok(())
}

/// Map a check-lane rejection to the validation variant, preserving the
/// runtime's exception class (`PGFError` vs `TypeError`) in the message.
fn check_err(error: &PyErr) -> GfError
{
    Python::attach(|py| {
        if error.is_instance_of::<PyTypeError>(py) {
            return GfError::Pgf(format!("TypeError: {error}"));
        }
        if error.is_instance_of::<PyException>(py) {
            // pgf.PGFError's message already carries the class name.
            return GfError::Pgf(error.to_string());
        }
        GfError::Python(error.to_string())
    })
}

/// Map a parse-lane rejection: the runtime's `ParseError` (the sentence does
/// not parse) goes to the validation variant; anything else is interop
/// failure.
fn parse_err(
    py: Python<'_>,
    error: &PyErr,
) -> GfError
{
    let is_parse = PyModule::import(py, "pgf")
        .and_then(|module| module.getattr("ParseError"))
        .is_ok_and(|class| error.is_instance(py, &class));
    if is_parse {
        return GfError::Pgf(error.to_string());
    }
    GfError::Python(error.to_string())
}
