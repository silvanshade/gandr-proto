//! The B′ `.gfd` surface: a tiny `GF`-expression builder with deterministic,
//! human-readable layout, and the matching reader.
//!
//! The printer owns the `.gfd` surface's readability rules (the same rules the
//! authoring guidance documents): flat applications stay on one line when they
//! fit; compound arguments are parenthesized and indented two columns under
//! their head; `Cons` chains flatten Lisp-style at constant indent with the
//! closing parens trailing. The reader accepts exactly what the printer emits
//! (any `GF` expression: atoms, string literals, parenthesized applications),
//! so corpus trees round-trip through Rust without a `GF` runtime in the
//! loop — this module is also the future `fmt` lane's engine.

use crate::error::GfError;

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

/// One surface token: an open paren, a close paren, or an atom (a bare
/// constant or a quoted string literal in its raw, still-escaped form).
enum Tok
{
    /// `(`.
    Open,
    /// `)`.
    Close,
    /// A self-delimiting token.
    Atom(String),
}

/// Parse one `.gfd` expression (the B′ surface).
///
/// The root application is naked (the printer emits `Head arg…` without
/// wrapping parens); every nested argument is parenthesized.
///
/// # Errors
/// [`GfError::Parse`] on an unterminated string, unbalanced parens, or an
/// application whose head is not an atom.
pub fn parse(text: &str) -> Result<Sexp, GfError>
{
    let toks = tokenize(text)?;
    // The implicit root frame collects the naked top-level application.
    let mut stack: Vec<Vec<Sexp>> = vec![Vec::new()];
    for tok in toks {
        match tok {
            | Tok::Open => stack.push(Vec::new()),
            | Tok::Atom(atom) => {
                let Some(frame) = stack.last_mut()
                else {
                    return Err(GfError::Parse("internal: no open frame".into()));
                };
                frame.push(Sexp::Atom(atom));
            },
            | Tok::Close => {
                if stack.len() == 1 {
                    return Err(GfError::Parse("unbalanced ')'".into()));
                }
                let Some(frame) = stack.pop()
                else {
                    return Err(GfError::Parse("internal: no open frame".into()));
                };
                let sexp = apply(frame)?;
                let Some(parent) = stack.last_mut()
                else {
                    return Err(GfError::Parse("internal: no open frame".into()));
                };
                parent.push(sexp);
            },
        }
    }
    if stack.len() != 1 {
        return Err(GfError::Parse("unbalanced '('".into()));
    }
    let Some(root) = stack.pop()
    else {
        return Err(GfError::Parse("internal: no open frame".into()));
    };
    apply(root)
}

/// Fold one completed frame into an application (the head must be an atom).
fn apply(frame: Vec<Sexp>) -> Result<Sexp, GfError>
{
    let mut iter = frame.into_iter();
    let Some(Sexp::Atom(head)) = iter.next()
    else {
        return Err(GfError::Parse(
            "an application's head is not an atom".into(),
        ));
    };
    Ok(Sexp::app(head, iter.collect()))
}

/// Tokenize the surface text (whitespace-separated, strings raw).
fn tokenize(text: &str) -> Result<Vec<Tok>, GfError>
{
    let mut toks = Vec::new();
    let mut chars = text.chars().peekable();
    loop {
        let Some(ch) = chars.next()
        else {
            break;
        };
        match ch {
            | _ if ch.is_whitespace() => {},
            | '(' => toks.push(Tok::Open),
            | ')' => toks.push(Tok::Close),
            | '"' => toks.push(Tok::Atom(string_literal(&mut chars)?)),
            | other => {
                let mut atom = String::from(other);
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace() || next == '(' || next == ')' {
                        break;
                    }
                    atom.push(next);
                    chars.next();
                }
                toks.push(Tok::Atom(atom));
            },
        }
    }
    Ok(toks)
}

/// Consume one string literal body (the opening quote is already consumed),
/// returning the raw, still-escaped, self-delimiting form.
fn string_literal(chars: &mut core::iter::Peekable<core::str::Chars<'_>>)
-> Result<String, GfError>
{
    let mut raw = String::from("\"");
    let mut escaped = false;
    loop {
        match chars.next() {
            | None => return Err(GfError::Parse("unterminated string literal".into())),
            | Some('\\') if !escaped => {
                raw.push('\\');
                escaped = true;
            },
            | Some('"') if !escaped => {
                raw.push('"');
                return Ok(raw);
            },
            | Some(other) => {
                raw.push(other);
                escaped = false;
            },
        }
    }
}

/// Unquote a string-literal atom (strip the quotes, resolve the B′ escapes
/// `\"` `\\` `\n` `\t` `\r`). Returns `None` for a bare (unquoted) atom.
#[must_use]
pub fn unquote(atom: &str) -> Option<String>
{
    let inner = atom.strip_prefix('"')?.strip_suffix('"')?;
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
                | Some('\\') => out.push('\\'),
                | Some(other) => {
                    out.push('\\');
                    out.push(other);
                },
                | None => out.push('\\'),
            },
            | Some(other) => out.push(other),
        }
    }
}
