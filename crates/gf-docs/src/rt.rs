//! The `GfRuntime` trait (the internalization seam) and its `PyO3` backend.
//!
//! Everything outside this module talks to the trait only; the `PyO3` backend
//! drives the `pgf` Python binding inside an embedded interpreter whose
//! environment is the uv project declared in this crate's `pyproject.toml`.

use pyo3::exceptions::PyException;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::error::GfDocsError;

/// The runtime surface the pipeline uses (proposal §4).
pub trait GfRuntime
{
    /// Read an expression (`.gfd` text), validate it at the `checkExpr` lane
    /// (`inferExpr`), and linearize it in the loaded concrete syntax.
    ///
    /// # Errors
    ///
    /// [`GfDocsError::Pgf`] when the runtime rejects the tree (unknown
    /// function or ill-typed application); [`GfDocsError::Python`] on
    /// interop failure.
    fn check_and_linearize(
        &self,
        expr: &str,
    ) -> Result<String, GfDocsError>;

    /// Validate without rendering (the negative-test lane).
    ///
    /// # Errors
    /// As [`GfRuntime::check_and_linearize`].
    fn check(
        &self,
        expr: &str,
    ) -> Result<(), GfDocsError>;
}

/// The `PyO3` backend: holds the loaded `pgf` module, PGF grammar object, and
/// selected concrete-syntax object.
pub struct PyPgf
{
    /// The imported `pgf` Python module (module-level `readExpr` lives here).
    module: Py<PyModule>,
    /// The loaded grammar (`pgf.readPGF` result).
    grammar: Py<PyAny>,
    /// The concrete syntax selected for linearization (`gr.languages[lang]`).
    concrete: Py<PyAny>,
}

impl PyPgf
{
    /// Load a compiled `.pgf` and select a concrete syntax by name.
    ///
    /// The crate's uv environment (`.venv` beside this manifest) is prepended
    /// to `sys.path` when present, so the binary needs no `PYTHONPATH`.
    ///
    /// # Errors
    ///
    /// [`GfDocsError::Python`] when the interpreter, the `pgf` import, the
    /// `.pgf` load, or the language lookup fails.
    #[inline]
    pub fn load(
        pgf_path: &str,
        language: &str,
    ) -> Result<Self, GfDocsError>
    {
        Python::attach(|py| {
            prepend_venv_site_packages(py)?;
            let module = PyModule::import(py, "pgf").map_err(|e| {
                let probe = py
                    .import("sys")
                    .and_then(|sys| {
                        let exe = sys.getattr("executable")?.extract::<String>()?;
                        let ver = sys.getattr("version")?.extract::<String>()?;
                        let path = sys.getattr("path")?.extract::<Vec<String>>()?;
                        Ok(format!(
                            "executable={exe} version={} path={path:?}",
                            ver.chars().take(6).collect::<String>()
                        ))
                    })
                    .unwrap_or_else(|_| "probe failed".to_owned());
                GfDocsError::Python(format!("{e} [{probe}]"))
            })?;
            let grammar = module
                .getattr("readPGF")
                .and_then(|f| f.call1((pgf_path,)))
                .map_err(|e| py_err(&e))?;
            let languages = grammar.getattr("languages").map_err(|e| py_err(&e))?;
            let concrete = languages.get_item(language).map_err(|e| py_err(&e))?;
            Ok(Self {
                module: module.unbind(),
                grammar: grammar.unbind(),
                concrete: concrete.unbind(),
            })
        })
    }

    /// Shared read-then-check body for the two public lanes.
    fn read_and_check(
        &self,
        expr: &str,
    ) -> Result<Py<PyAny>, GfDocsError>
    {
        Python::attach(|py| {
            let tree = self
                .module
                .bind(py)
                .getattr("readExpr")
                .and_then(|f| f.call1((expr,)))
                .map_err(|e| py_err(&e))?;
            let grammar = self.grammar.bind(py);
            let (checked, _ty): (Py<PyAny>, Py<PyAny>) = grammar
                .getattr("inferExpr")
                .and_then(|f| f.call1((tree,)))
                .and_then(|t| t.extract())
                .map_err(|e| check_err(&e))?;
            Ok(checked)
        })
    }
}

impl GfRuntime for PyPgf
{
    #[inline]
    fn check_and_linearize(
        &self,
        expr: &str,
    ) -> Result<String, GfDocsError>
    {
        let checked = self.read_and_check(expr)?;
        Python::attach(|py| {
            self.concrete
                .bind(py)
                .getattr("linearize")
                .and_then(|f| f.call1((checked,)))
                .and_then(|s| s.extract::<String>())
                .map_err(|e| py_err(&e))
        })
    }

    #[inline]
    fn check(
        &self,
        expr: &str,
    ) -> Result<(), GfDocsError>
    {
        self.read_and_check(expr).map(|_| ())
    }
}

/// Map an interop-side failure (import, attribute, conversion) to the
/// interop variant.
fn py_err(error: &PyErr) -> GfDocsError
{
    GfDocsError::Python(error.to_string())
}

/// Prepend the crate's uv `site-packages` to `sys.path` when the directory
/// exists (the `.venv` lives beside this crate's manifest at build time).
fn prepend_venv_site_packages(py: Python<'_>) -> Result<(), GfDocsError>
{
    let lib = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".venv/lib");
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
fn check_err(error: &PyErr) -> GfDocsError
{
    Python::attach(|py| {
        if error.is_instance_of::<PyTypeError>(py) {
            return GfDocsError::Pgf(format!("TypeError: {error}"));
        }
        if error.is_instance_of::<PyException>(py) {
            // pgf.PGFError's message already carries the class name.
            return GfDocsError::Pgf(error.to_string());
        }
        GfDocsError::Python(error.to_string())
    })
}
