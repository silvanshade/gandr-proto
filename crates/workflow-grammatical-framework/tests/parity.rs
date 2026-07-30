//! Wrapper ↔ `Python`-`API` parity: each nominal wrapper forwards one-for-one
//! to the live `pgf` binding object it holds (the vkpp pattern, crate docs),
//! so every method's result must equal the direct binding call's result in
//! the same interpreter. Each test drives both sides.
//!
//! The lanes skip cleanly when the `GF` environment is absent (no compiled
//! PGF or no pgf-enabled Python), so the suite stays green on a bare
//! checkout; the mise corpus arc provisions the environment and exercises
//! them for real.

use core::error::Error;
extern crate alloc;

use alloc::collections::BTreeMap;
use std::path::PathBuf;

use gandr_workflow_grammatical_framework::rt::CategoryName;
use gandr_workflow_grammatical_framework::rt::ExprText;
use gandr_workflow_grammatical_framework::rt::ExprUnpack;
use gandr_workflow_grammatical_framework::rt::LanguageName;
use gandr_workflow_grammatical_framework::rt::PgfConcrete;
use gandr_workflow_grammatical_framework::rt::PgfExpr;
use gandr_workflow_grammatical_framework::rt::PgfGrammar;
use gandr_workflow_grammatical_framework::rt::PgfModule;
use pyo3::Bound;
use pyo3::PyAny;
use pyo3::Python;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyModule;
use pyo3::types::PyTuple;

/// Shared result type for the parity witnesses.
type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// The compiled docs grammar the corpus environment produces.
fn pgf_path() -> PathBuf
{
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/gf/GandrDocsLex.pgf")
}

/// The wrapper handles, or `None` when the `GF` environment is unprovisioned.
/// Importing the module first also puts the uv environment's site-packages on
/// the interpreter's path, so the raw side's `import pgf` rides the same fix.
fn handles() -> Option<(PgfModule, PgfGrammar, PgfConcrete)>
{
    let pgf = pgf_path();
    if !pgf.exists() {
        return None;
    }
    let module = PgfModule::import().ok()?;
    let grammar = module.read_pgf(&pgf).ok()?;
    let concrete = grammar
        .language(&LanguageName::new("GandrDocsLexHtml"))
        .ok()?;
    Some((module, grammar, concrete))
}

/// A portable deconstruction of an expression tree (the parity currency:
/// both sides reduce to this before comparison).
#[derive(Debug, Eq, PartialEq)]
enum Tree
{
    /// A string literal.
    Lit(String),
    /// A constructor application.
    App(String, Vec<Self>),
}

/// One deconstructed node (the fold's per-level shape): a string literal or
/// a constructor application.
enum Node<N>
{
    /// A string literal.
    Lit(String),
    /// A constructor application.
    App(String, Vec<N>),
}

/// Fold an expression into the portable tree with an explicit work stack
/// (the house traversal pattern: `Enter` decomposes one node, `Complete`
/// folds converted children; arguments push in reverse for source order).
///
/// # Termination
/// - reason: an explicit work stack decomposes each expression node into strict
///   children, visited once.
/// - measure: pending frames; every `Complete` pops after exactly its
///   children's conversions.
/// - boundedness: the source expression is a finite tree.
/// - input recursion: none.
fn fold_tree<N, U>(
    root: N,
    mut unpack: U,
) -> TestResult<Tree>
where
    U: FnMut(N) -> TestResult<Node<N>>,
{
    /// One work-stack frame.
    enum Frame<N>
    {
        /// Decompose this node.
        Enter(N),
        /// Record a string literal.
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

    let mut done: Vec<Tree> = Vec::new();
    let mut stack = vec![Frame::Enter(root)];
    while let Some(frame) = stack.pop() {
        match frame {
            | Frame::Enter(node) => match unpack(node)? {
                | Node::Lit(text) => stack.push(Frame::Literal(text)),
                | Node::App(head, args) => {
                    stack.push(Frame::Complete {
                        head,
                        arity: args.len(),
                    });
                    for arg in args.into_iter().rev() {
                        stack.push(Frame::Enter(arg));
                    }
                },
            },
            | Frame::Literal(text) => done.push(Tree::Lit(text)),
            | Frame::Complete { head, arity } => {
                if arity == 0 {
                    done.push(Tree::App(head, vec![]));
                    continue;
                }
                let children = done.split_off(done.len().saturating_sub(arity));
                done.push(Tree::App(head, children));
            },
        }
    }
    done.pop()
        .ok_or_else(|| "expression conversion produced no tree".into())
}

/// Deconstruct through the wrapper's `unpack`.
fn wrapper_tree(expr: PgfExpr) -> TestResult<Tree>
{
    fold_tree(expr, |node| {
        Ok(match node.unpack()? {
            | ExprUnpack::Literal(text) => Node::Lit(text),
            | ExprUnpack::Application { head, args } => Node::App(head, args),
            | other => return Err(format!("unexpected unpack variant: {other:?}").into()),
        })
    })
}

/// Deconstruct through the raw binding's `unpack`.
fn raw_tree(expr: Bound<'_, PyAny>) -> TestResult<Tree>
{
    fold_tree(expr, |node| {
        let unpacked = node.call_method0("unpack")?;
        if let Ok(text) = unpacked.extract::<String>() {
            return Ok(Node::Lit(text));
        }
        let tuple = unpacked
            .cast_into::<PyTuple>()
            .map_err(|error| error.to_string())?;
        let head = tuple.get_item(0)?.extract::<String>()?;
        let args = tuple
            .get_item(1)?
            .cast_into::<PyList>()
            .map_err(|error| error.to_string())?;
        Ok(Node::App(head, args.iter().collect()))
    })
}

/// The parity witnesses: each test drives the wrapper and the raw binding
/// against the same interpreter and compares the reduced results.
#[cfg(test)]
mod tests
{
    use super::*;

    /// `readExpr` + `unpack`: the wrapper and the raw binding deconstruct
    /// the same source to the same tree (literal, application, nesting).
    #[test]
    fn read_expr_unpack_parity() -> TestResult
    {
        let Some((module, _grammar, _concrete)) = handles()
        else {
            return Ok(());
        };
        let source =
            "ConsInline (Txt \"the\") (ConsInlineGlued (CodeInline \"def rec\") BaseInline)";
        let wrapped = module.read_expr(&ExprText::new(source))?;
        let wrapper_side = wrapper_tree(wrapped)?;
        let raw_side = Python::attach(|py| {
            let pgf = PyModule::import(py, "pgf")?;
            let expr = pgf.getattr("readExpr")?.call1((source,))?;
            raw_tree(expr)
        })?;
        assert_eq!(wrapper_side, raw_side);
        assert_eq!(
            wrapper_side,
            Tree::App("ConsInline".to_owned(), vec![
                Tree::App("Txt".to_owned(), vec![Tree::Lit("the".to_owned())]),
                Tree::App("ConsInlineGlued".to_owned(), vec![
                    Tree::App("CodeInline".to_owned(), vec![Tree::Lit(
                        "def rec".to_owned()
                    )]),
                    Tree::App("BaseInline".to_owned(), vec![]),
                ],),
            ],),
        );
        Ok(())
    }

    /// `functionsByCat`: the wrapper returns exactly the binding's function
    /// list for a category (contents and order).
    #[test]
    fn functions_by_cat_parity() -> TestResult
    {
        let Some((_module, grammar, _concrete)) = handles()
        else {
            return Ok(());
        };
        let wrapper_side: Vec<String> = grammar
            .functions_by_cat(&CategoryName::new("Inline"))?
            .iter()
            .map(|name| name.as_ref().to_owned())
            .collect();
        let raw_side: Vec<String> = Python::attach(|py| -> TestResult<Vec<String>> {
            let pgf = PyModule::import(py, "pgf")?;
            let gr = pgf
                .getattr("readPGF")?
                .call1((pgf_path().to_string_lossy().as_ref(),))?;
            let list = gr
                .call_method1("functionsByCat", ("Inline",))?
                .cast_into::<PyList>()
                .map_err(|error| error.to_string())?;
            let mut names = Vec::with_capacity(list.len());
            for item in list.iter() {
                names.push(item.extract::<String>()?);
            }
            Ok(names)
        })?;
        assert_eq!(wrapper_side, raw_side);
        for expected in [
            "Txt",
            "Bold",
            "Italic",
            "CodeInline",
            "MathInline",
            "TermRef",
            "XRef",
        ] {
            assert!(
                wrapper_side.iter().any(|name| name == expected),
                "Inline inventory missing {expected}"
            );
        }
        Ok(())
    }

    /// `tabularLinearize`: the wrapper's record map equals the binding's
    /// field dict for the same expression.
    #[test]
    fn tabular_linearize_parity() -> TestResult
    {
        let Some((module, _grammar, concrete)) = handles()
        else {
            return Ok(());
        };
        let expr = module.read_expr(&ExprText::new("term_component"))?;
        let wrapper_side = concrete.tabular_linearize(&expr)?;
        let raw_side: BTreeMap<String, String> =
            Python::attach(|py| -> TestResult<BTreeMap<String, String>> {
                let pgf = PyModule::import(py, "pgf")?;
                let gr = pgf
                    .getattr("readPGF")?
                    .call1((pgf_path().to_string_lossy().as_ref(),))?;
                let eng = gr.getattr("languages")?.get_item("GandrDocsLexHtml")?;
                let expr = pgf.getattr("readExpr")?.call1(("term_component",))?;
                let table = eng
                    .call_method1("tabularLinearize", (expr,))?
                    .cast_into::<PyDict>()
                    .map_err(|error| error.to_string())?;
                let mut map = BTreeMap::new();
                for (key, value) in table.iter() {
                    map.insert(key.extract::<String>()?, value.extract::<String>()?);
                }
                Ok(map)
            })?;
        assert_eq!(&*wrapper_side, &raw_side);
        assert!(
            raw_side.contains_key("text"),
            "term records carry a text field"
        );
        Ok(())
    }

    /// `inferExpr`: the wrapper accepts what the binding accepts (and the
    /// checked tree linearizes identically) and rejects what the binding
    /// rejects (an arity-mismatched application).
    #[test]
    fn infer_expr_accept_reject_parity() -> TestResult
    {
        let Some((module, grammar, concrete)) = handles()
        else {
            return Ok(());
        };
        // accept: a well-typed application; both sides check and linearize.
        let good = module.read_expr(&ExprText::new("Txt \"ok\""))?;
        let checked = grammar.infer_expr(&good)?;
        let wrapper_lin = concrete.linearize(&checked)?;
        let raw_lin: String = Python::attach(|py| -> TestResult<String> {
            let pgf = PyModule::import(py, "pgf")?;
            let gr = pgf
                .getattr("readPGF")?
                .call1((pgf_path().to_string_lossy().as_ref(),))?;
            let eng = gr.getattr("languages")?.get_item("GandrDocsLexHtml")?;
            let expr = pgf.getattr("readExpr")?.call1(("Txt \"ok\"",))?;
            let checked = gr.call_method1("inferExpr", (expr,))?.get_item(0_usize)?;
            Ok(eng
                .call_method1("linearize", (checked,))?
                .extract::<String>()?)
        })?;
        assert_eq!(wrapper_lin, raw_lin);
        // reject: Txt wants a Str; Bold BaseInline is an Inline — a
        // genuinely ill-typed application (a missing argument alone is just
        // a curried partial application, which checks fine). Both sides
        // must refuse.
        let bad = module.read_expr(&ExprText::new("Txt (Bold BaseInline)"))?;
        let wrapper_err = grammar.infer_expr(&bad);
        assert!(wrapper_err.is_err(), "wrapper accepted an ill-typed tree");
        let raw_rejected = Python::attach(|py| -> TestResult<bool> {
            let pgf = PyModule::import(py, "pgf")?;
            let gr = pgf
                .getattr("readPGF")?
                .call1((pgf_path().to_string_lossy().as_ref(),))?;
            let expr = pgf.getattr("readExpr")?.call1(("Txt (Bold BaseInline)",))?;
            Ok(gr.call_method1("inferExpr", (expr,)).is_err())
        })?;
        assert!(raw_rejected, "binding accepted an ill-typed tree");
        Ok(())
    }
}
