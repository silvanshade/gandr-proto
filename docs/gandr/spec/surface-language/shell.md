# The shell fragment

The shell face of the surface: shell blocks and jobs, the embedded shell sub-grammar, the host escape, string interpolation, and the REPL split.
The interactive-usage surface it serves — the loop, history, completion, the language-server adapter, the formatter — is the interactive-surface design (now in the project's research vault — the corpus README's migration banner), and the host-effect seam it runs through is [[../implementation#The runtime host|the implementation track's]].
The deferred semantic shell DSL (job control, byte-stream sessions, worlds) is designed direction; its typed reading is below.

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

The deferred POSIX tail — command environment assignments (`FOO=bar cmd`), command negation (`! cmd`), process substitution (`<( … )`), job control and history — parses-and-declines or lexes as ordinary words today, each a later shell-stage widening; environment assignment in particular already molds as its own `environment_assignment` atom, so only the prefix's binding semantics is pending, and that is wanted working for the daily-driver rung.

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

**That design is not a shell-shaped bolt-on: it is an effect signature, session protocols, and the control operators, with no shell-specific typing rules at all.** Every row below is an **elaboration** rather than a primitive, which is the whole claim — POSIX is being read as an instance of machinery the calculus already needs.

| POSIX construct           | typed reading                                                                                                                          |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| process, `exec`           | a shell effect operation spawning a computation at a job world, with the three standard streams as endpoints in the linear zone        |
| pipe `p \| q`             | a **fork** of a byte-stream session `Pipe = μX. ⊕{ chunk: !Bytes.X, eof: end }` — one side holds the sending end, the other its dual   |
| redirection `> f`, `2>&1` | **endpoint delegation** — rebinding which channel a job's stream names refer to                                                        |
| exit status               | a returner over unit-or-exit-code; abort-on-error is a **handler policy** on the failure operation, not a mode flag                    |
| signal                    | an **asynchronous effect** delivered as an interrupt [@ahman-pretnar-2021-asynchronous-effects]; a trap is an installed handler clause |
| suspension                | **capture the job's stack** — a `shift` at the job delimiter, yielding a linear stack value that owns the job's linear zone            |
| jobs table, `bg`, `fg`    | a registry of captured linear stacks: foreground is resume **with** the terminal capability, background is resume **without** it       |
| job-control groups        | capability-scoped — the terminal is a linear capability, and only the foreground job holds it                                          |
| subshell                  | a child world, or a plain fork where no isolation is needed                                                                            |
| remote execution          | migration of the shell computation to another world — remote execution **is** the worlds feature rather than an addition to it         |
| word expansion, globbing  | macro-phase computation: expansion happens before the job's runtime phase, exactly as in real shells                                   |

Two observations are what make the mapping more than a curiosity.

**Job control literally is first-class one-shot stacks.** Suspend is capture; foreground and background are resume with and without a capability; a terminated job holding open pipes is exactly the unwind-obligation discipline the linear zone already owes.
The shell is the **motivating application** for first-class stacks, not a beneficiary of them.

**Every shell footgun lands on a static discipline.** A dangling pipe becomes a linear-zone obligation, a signal-unsafe trap becomes handler typing, a double foreground becomes a linear capability, and a remote-execution data race becomes mobility.
The interactive payoff follows: a derivation surface that shows a pipeline's session protocol advancing, a suspended job as an inspectable stack value, and a trap as a handler frame is a genuinely novel way to **teach** POSIX, not merely to run it.

The fragment graduates into the real handler as the linear-zone runtime, the process model, and worlds land.
The calculus every row above elaborates into — the operation and handle rules, the stack judgment, and the one-shot linearity that makes a terminated job's obligations a typing matter — is [[../implementation/effects-and-control]].
What is true **today**, marked as such: the host operation families `Exec`/`Fs`/`Proc`/`Env` run through the preserved host-effect seam; pipelines at the REPL parse but are **declined past the parse** with a named diagnostic; the surface `m.op(a)` member-call elaborates to `perform m.op { … }`.

## Source and confidence

The fragment as built is verified against the grammar crate's shell rules and the W4e fold-in record (braced parameter as a distinct mode, double-quoted words as one atom, bracketed subshells, fd redirections, the coverage table of folded versus deferred POSIX forms) — high confidence.
The REPL split is the driver-split decision of record (bare `gandr` = shell-REPL, `gandr tui` = programming environment), and the host-escape bounds are the landed standalone-word cut.
The POSIX-to-typed mapping above is designed direction throughout, absorbed here in full; the interactive surface it serves is carried at depth in the project's research vault (the corpus README's migration banner).
The deferred semantic DSL (job control, byte-stream sessions, the `Shell` handler) is designed direction, marked wherever mentioned.
