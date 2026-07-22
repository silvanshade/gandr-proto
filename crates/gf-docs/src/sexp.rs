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

    /// Render at the given indent, iteratively (explicit work stack, no
    /// recursion): nodes render in document order, argument positions
    /// parenthesize compound applications, and `Cons` chains flatten
    /// Lisp-style at constant indent.
    fn render_at(
        &self,
        indent: usize,
        out: &mut String,
    )
    {
        /// One pending layout step.
        enum Task<'tree>
        {
            /// Render an expression at an indent.
            Node(&'tree Sexp, usize),
            /// Render an argument (parenthesized when compound).
            Arg(&'tree Sexp, usize),
            /// Emit a newline and indent padding.
            Break(usize),
            /// Emit a closing parenthesis.
            Close,
        }

        let mut stack = vec![Task::Node(self, indent)];
        while let Some(task) = stack.pop() {
            match task {
                | Task::Close => out.push(')'),
                | Task::Break(level) => {
                    out.push('\n');
                    pad(out, level);
                },
                | Task::Arg(sexp, level) => match sexp {
                    | &Self::Atom(_) => stack.push(Task::Node(sexp, level)),
                    | &Self::App { .. } => {
                        out.push('(');
                        stack.push(Task::Close);
                        stack.push(Task::Node(sexp, level.saturating_add(1)));
                    },
                },
                | Task::Node(sexp, level) => match sexp {
                    | &Self::Atom(ref text) => out.push_str(text),
                    | &Self::App { ref head, ref args } => {
                        if let Some(flat) = flat_form(head, args) {
                            out.push_str(&flat);
                            continue;
                        }
                        out.push_str(head);
                        if head.starts_with("Cons") {
                            // Lisp list style: element on the head's line, tail
                            // at the same indent, closing parens trailing.
                            let mut iter = args.iter();
                            if let Some(element) = iter.next() {
                                out.push(' ');
                                if let Some(tail) = iter.next() {
                                    stack.push(Task::Arg(tail, level.saturating_add(1)));
                                    stack.push(Task::Break(level.saturating_add(1)));
                                }
                                stack.push(Task::Arg(element, level.saturating_add(1)));
                            }
                            continue;
                        }
                        for arg in args.iter().rev() {
                            stack.push(Task::Arg(arg, level.saturating_add(2)));
                            stack.push(Task::Break(level.saturating_add(2)));
                        }
                    },
                },
            }
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
