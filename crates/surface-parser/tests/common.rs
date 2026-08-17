//! Shared fixtures for the `gandr-surface-parser` integration suites.

use std::sync::LazyLock;

use gandr_surface_grammar::Pbg;
use gandr_surface_grammar::built_in;

/// The real built-in grammar, built once per test process and shared across
/// the suites in this funnel binary.
///
/// Nextest runs every test in its own process, so one cache here is the
/// per-process floor — a second cache elsewhere in the funnel would double
/// the ~100ms `built_in()` cost a grammar-touching test pays.
pub fn built() -> &'static Pbg
{
    /// The process-wide cached grammar.
    static BUILT_IN: LazyLock<Pbg> =
        LazyLock::new(|| built_in().expect("built-in grammar assembles"));
    &BUILT_IN
}
