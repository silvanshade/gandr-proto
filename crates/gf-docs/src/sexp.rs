//! A tiny `GF`-expression builder with deterministic, human-readable layout.
//!
//! The printer owns the `.gfd` surface's readability rules (the same rules the
//! authoring guidance documents): flat applications stay on one line when they
//! fit; compound arguments are parenthesized and indented two columns under
//! their head; `Cons` chains flatten Lisp-style at constant indent with the
//! closing parens trailing. This module is also the future `fmt` lane's engine.

/// A `GF` expression under construction.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Sexp
{
    /// A self-delimiting token: a constant or a quoted string literal.
    Atom(String),
    /// A constructor application (the head is the constructor name).
    App
    {
        /// The constructor name.
        head: String,
        /// The argument expressions.
        args: Vec<Self>,
    },
}

impl Sexp
{
    /// Build an atom.
    #[inline]
    #[must_use]
    pub fn atom<T>(text: T) -> Self
    where
        T: Into<String>,
    {
        Self::Atom(text.into())
    }

    /// Build a constructor application.
    #[inline]
    #[must_use]
    pub fn app<T>(
        head: T,
        args: Vec<Self>,
    ) -> Self
    where
        T: Into<String>,
    {
        Self::App {
            head: head.into(),
            args,
        }
    }

    /// Render with the canonical layout at zero indent.
    #[inline]
    #[must_use]
    pub fn render(&self) -> String
    {
        let mut out = String::new();
        self.render_at(0, &mut out);
        out
    }

    /// Render at the given indent; compound applications as arguments are
    /// parenthesized by [`Sexp::render_arg`].
    fn render_at(
        &self,
        indent: usize,
        out: &mut String,
    )
    {
        match *self {
            | Self::Atom(ref text) => out.push_str(text),
            | Self::App { ref head, ref args } => {
                if let Some(flat) = flat_form(head, args) {
                    out.push_str(&flat);
                    return;
                }
                out.push_str(head);
                if head.starts_with("Cons") {
                    // Lisp list style: element, then the tail at the same indent.
                    let mut iter = args.iter();
                    if let Some(element) = iter.next() {
                        out.push(' ');
                        element.render_arg(indent.saturating_add(1), out);
                    }
                    if let Some(tail) = iter.next() {
                        out.push('\n');
                        pad(out, indent.saturating_add(1));
                        tail.render_arg(indent.saturating_add(1), out);
                    }
                    return;
                }
                for arg in args {
                    out.push('\n');
                    pad(out, indent.saturating_add(2));
                    arg.render_arg(indent.saturating_add(2), out);
                }
            },
        }
    }

    /// Render as an argument (parenthesized when compound).
    fn render_arg(
        &self,
        indent: usize,
        out: &mut String,
    )
    {
        match *self {
            | Self::Atom(_) => self.render_at(indent, out),
            | Self::App { .. } => {
                out.push('(');
                self.render_at(indent.saturating_add(1), out);
                out.push(')');
            },
        }
    }
}

/// The single-line form when every argument is atomic and it fits.
fn flat_form(
    head: &str,
    args: &[Sexp],
) -> Option<String>
{
    /// The column budget for one-line applications.
    const FLAT_LIMIT: usize = 72;
    let mut width = head.len();
    let mut flat = String::from(head);
    for arg in args {
        let Sexp::Atom(ref text) = *arg
        else {
            return None;
        };
        width = width.saturating_add(text.len()).saturating_add(1);
        if width > FLAT_LIMIT {
            return None;
        }
        flat.push(' ');
        flat.push_str(text);
    }
    Some(flat)
}

/// Append `count` spaces.
fn pad(
    out: &mut String,
    count: usize,
)
{
    for _ in 0 .. count {
        out.push(' ');
    }
}
