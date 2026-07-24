//! The B′ `.gfd` surface: the crate's expression tree type and its canonical
//! printer (deterministic, human-readable layout).
//!
//! The printer owns the `.gfd` surface's readability rules (the same rules the
//! authoring guidance documents): flat applications stay on one line when they
//! fit; compound arguments are parenthesized and indented two columns under
//! their head; `Cons` chains flatten Lisp-style at constant indent with the
//! closing parens trailing. It is the future `fmt` lane's engine (`gandr-hz8`).
//! Trees are never *read* here — reading is the `GF` runtime's (`readExpr`,
//! reached via [`crate::rt`]); the printer is the one blessed surface exception
//! because the `GF` toolchain ships no formatting or canonical-layout tooling
//! (docs/workflow/gfd.md §"The bindings-first doctrine").

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

/// Borrowed text for one quoted S-expression atom.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct AtomText<'text>(&'text str);

impl<'text> From<&'text str> for AtomText<'text>
{
    #[inline]
    fn from(text: &'text str) -> Self
    {
        Self(text)
    }
}

impl<'text> From<&'text String> for AtomText<'text>
{
    #[inline]
    fn from(text: &'text String) -> Self
    {
        Self(text.as_str())
    }
}

/// Current indentation level while rendering an S-expression.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct LayoutIndent(usize);

impl LayoutIndent
{
    /// Root indentation.
    const ROOT: Self = Self(0);

    /// Indentation for the next application position.
    fn continuation(self) -> Self
    {
        Self(self.0.saturating_add(1))
    }

    /// Indentation for a nested application argument.
    fn nested(self) -> Self
    {
        Self(self.0.saturating_add(2))
    }
}

/// Borrowed application-head text during flat rendering.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct ApplicationHead<'text>(&'text str);

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

    /// Build a string-literal atom from its unquoted text (the [`quote`]d form
    /// is stored; [`unquote`] resolves it back).
    #[inline]
    #[must_use]
    pub fn string<T>(text: T) -> Self
    where
        T: Into<String>,
    {
        Self::atom(quote(text.into()))
    }

    /// Render with the canonical layout at zero indent.
    #[inline]
    #[must_use]
    pub fn render(&self) -> String
    {
        let mut out = String::new();
        self.render_at(LayoutIndent::ROOT, &mut out);
        out
    }

    /// Render at the given indent, iteratively (explicit work stack, no
    /// recursion): nodes render in document order, argument positions
    /// parenthesize compound applications, and `Cons` chains flatten
    /// Lisp-style at constant indent.
    fn render_at(
        &self,
        indent: LayoutIndent,
        out: &mut String,
    )
    {
        /// One pending layout step.
        enum Task<'tree>
        {
            /// Render an expression at an indent.
            Node(&'tree Sexp, LayoutIndent),
            /// Render an argument (parenthesized when compound).
            Arg(&'tree Sexp, LayoutIndent),
            /// Emit a newline and indent padding.
            Break(LayoutIndent),
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
                        stack.push(Task::Node(sexp, level.continuation()));
                    },
                },
                | Task::Node(sexp, level) => match sexp {
                    | &Self::Atom(ref text) => out.push_str(text),
                    | &Self::App { ref head, ref args } => {
                        if let Some(flat) = flat_form(ApplicationHead(head.as_str()), args) {
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
                                    stack.push(Task::Arg(tail, level.continuation()));
                                    stack.push(Task::Break(level.continuation()));
                                }
                                stack.push(Task::Arg(element, level.continuation()));
                            }
                            continue;
                        }
                        for arg in args.iter().rev() {
                            stack.push(Task::Arg(arg, level.nested()));
                            stack.push(Task::Break(level.nested()));
                        }
                    },
                },
            }
        }
    }
}

/// The single-line form when every argument is atomic and it fits.
fn flat_form(
    head: ApplicationHead<'_>,
    args: &[Sexp],
) -> Option<String>
{
    /// The column budget for one-line applications.
    const FLAT_LIMIT: usize = 72;
    let mut width = head.0.len();
    let mut flat = String::from(head.0);
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
    count: LayoutIndent,
)
{
    for _ in 0 .. count.0 {
        out.push(' ');
    }
}

/// Unquote a string-literal atom
/// the result is self-delimiting with `\"` `\\` `\n` `\t` `\r` escaped.
#[inline]
#[must_use]
pub fn quote<T>(text: T) -> String
where
    T: Into<String>,
{
    let text = text.into();
    let mut out = String::with_capacity(text.len().saturating_add(2));
    out.push('"');
    for ch in text.chars() {
        match ch {
            | '"' => out.push_str("\\\""),
            | '\\' => out.push_str("\\\\"),
            | '\n' => out.push_str("\\n"),
            | '\t' => out.push_str("\\t"),
            | '\r' => out.push_str("\\r"),
            | other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Unquote a string-literal atom (strip the quotes, resolve the B′ escapes
/// `\"` `\\` `\n` `\t` `\r`). Returns `None` for a bare (unquoted) atom.
#[inline]
#[must_use]
pub fn unquote(atom: AtomText<'_>) -> Option<String>
{
    let atom = atom.0;
    let stripped = atom.strip_prefix('"')?;
    let inner = stripped.strip_suffix('"')?;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    loop {
        match chars.next() {
            | None => return Some(out),
            | Some('\\') => match chars.next() {
                | Some('n') => out.push('\n'),
                | Some('t') => out.push('\t'),
                | Some('r') => out.push('\r'),
                | Some('"') => out.push('"'),
                | Some('\\') | None => out.push('\\'),
                | Some(other) => {
                    out.push('\\');
                    out.push(other);
                },
            },
            | Some(other) => out.push(other),
        }
    }
}
