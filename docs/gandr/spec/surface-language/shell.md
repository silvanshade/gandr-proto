# The shell fragment

The shell face of the surface: shell blocks and jobs, the embedded shell sub-grammar, the host escape, string interpolation, and the REPL split.
The interactive-usage surface it serves (REPL, history, completions, LSP, formatter) and the host-effect seam (`Exec`/`Fs`/`Proc`/`Env`) are the [[../implementation#The runtime host|implementation track]]; the deferred semantic shell DSL (job control, byte-stream sessions, worlds) is the late-schedule design lane.

## Shell blocks and jobs

```text
def build = thunk {
  #!{
    mkdir -p out;
    echo "building for $USER" > out/log;
    [ cd out; make all ];
    $(notify("build finished"))
  }
}
```

* `#!{ … }` is a **shell block**; `#!dialect{ … }` (e.g. `#!zsh{ … }`) tags a dialect.
  A first-line `#!/…` shebang stays recognized as a comment, so gandr scripts remain executable.
* A job kept as a value is a **thunk over a shell block** — jobs are values; running them is a computation.
* The block's type lives in the effect-row reading: a shell computation carries a shell effect row, and the reserved `Shell` name (distinct from the v0 host operation families `Exec`/`Fs`/`Proc`/`Env`) is the late-schedule home of the real shell handler.

## The embedded sub-grammar

The shell context is an embedded, boundary-resilient sub-grammar with prefix discrimination — a single token (`#!{`, `$!{`) tells the parser and editors which grammar is active with zero lookahead:

* **lists**: pipelines joined by `;`, `&`, or newline;
* **pipelines**: `cmd | cmd | …` (pipe tightest), with `&&` and `||` above it;
* **commands**: a name plus arguments, strings, and redirections;
* **quoting**: single-quote runs; double-quoted strings lex as **one atom** (fragments, escapes, and expansions inside — interior spaces preserved, never juxtaposed words);
* **expansions**: `$name` and the braced `${name}` — parameter expansion, a **distinct labeler mode** from string interpolation: a shell parameter name is not a gandr binding;
* **command substitution**: `$!{ … }` (and `$!dialect{ … }`), shell-internal;
* **subshell**: `[ … ]` — square brackets, so POSIX parentheses stay free for the host escape (a recorded divergence from POSIX spelling);
* **redirections**: `<`, `>`, `>>`, `<&`, `>&`, `<>`, with file-descriptor prefixes — `2> err.log`, `2>&1`;
* **host escape**: `$( E )`, whose interior is an ordinary gandr expression (below).

The deferred POSIX tail — command environment assignments (`FOO=bar cmd`), command negation (`! cmd`), process substitution (`<( … )`), job control and history — parses-and-declines or lexes as ordinary words today, each a later shell-stage widening; environment assignment in particular is currently unmoldable and wanted working for the daily-driver rung.

## The host escape `$( E )`

The boundary where gandr values enter shell commands, with a recorded tension kept honestly: POSIX reads `$( )` as command substitution and gandr reads it as the opposite (host escape) — the allocation survives because `$!{ }` _is_ command substitution.

As built, the standalone-word escape is bounded and safe:

* the escape must be a standalone lexical word (mixed-word interpolation is a named deferral);
* its interior is type-checked before dispatch;
* evaluation is **exactly once**, left-to-right within the command, producing one `String` bind per escape;
* the result contributes **one argv element** — no split, no reparse;
* malformed interiors recover strictly and totally, with exact origin/payload paths.

## String interpolation

Expression-position strings carry interpolation segments:

```text
def msg = "built ${count} targets"
```

* The `${ E }` opener and `}` closer are the labeler's string-segment boundaries; the interior is a gandr expression.
* Interpolation lives **only in expression-position strings** — a shell double-quoted word is one atom with expansions, a different mechanism (`${name}` parameter expansion is not `${E}` interpolation).
* No format specifiers at the current rung.

## The REPL split

The driver's two faces are decided:

* **Bare `gandr` on a terminal is the minimal shell-REPL** — the daily-driver shell; `gandr tui` is the explicit programming environment.
* REPL input is implicitly inside the shell fragment: bare lines are shell commands; **keyword-led lines are host items**; a leading `^` forces a command where a keyword would shadow it.
* The parser's obligation queries drive the loop: `expected()` decides continuation lines and renders hints, and obligations materialize at execute points, so the user always sees how the system chose to repair.
* The exec spawn mode is inherit-stdio: bare discarded REPL lines inherit the terminal's stdio, while consumed replies stay captured and typed.
* The session owns `cwd`/`env`; command history is content-addressed.

## The POSIX-to-typed mapping, and the deferred DSL

The shell fragment is the embedding surface; the semantic shell language is a separate, later design.
The mapping of record sketches the direction: a pipe is a forked byte-stream protocol (`Pipe = μX. ⊕{chunk: !Bytes.X, eof: end}`), job control is one-shot stacks, terminal capability is a located capability, and the fragment graduates into the real handler as the linear-Σ runtime, the process model, and worlds land.
What is true **today**, marked as such: the host operation families `Exec`/`Fs`/`Proc`/`Env` run through the preserved host-effect seam; pipelines at the REPL parse but are **declined past the parse** with a named diagnostic; the surface `m.op(a)` member-call elaborates to `perform m.op { … }`.

## Source and confidence

The fragment as built is verified against the grammar crate's shell rules and the W4e fold-in record (braced parameter as a distinct mode, double-quoted words as one atom, bracketed subshells, fd redirections, the coverage table of folded versus deferred POSIX forms) — high confidence.
The REPL split is the driver-split decision of record (bare `gandr` = shell-REPL, `gandr tui` = programming environment); the host-escape bounds are the landed standalone-word cut; the usage-surface and effects proposals carry the REPL/typed-mapping design at full depth, and the manual's effects chapter restates the typed reading.
The deferred semantic DSL (job control, byte-stream sessions, the `Shell` handler) is designed direction, marked wherever mentioned.
