import { type Plugin, type Rule, RuleConfigSeverity, type UserConfig } from "@commitlint/types";

// ── Canonical agent co-author registry ───────────────────────────────────────
// A `Co-Authored-By` whose email host is a machine-agent provider must match one
// of the canonical trailer strings below BYTE-FOR-BYTE (key casing included).
// Human co-authors (any other host) are unconstrained beyond a valid shape. New
// models/variants require an explicit entry here — strict by design, to keep
// agent attribution uniform across the publishable history. Agent attribution
// itself is project-concern (honest provenance); extend the registry upstream
// in agentic-dev, not per consumer.
const AGENT_EMAIL_HOSTS: readonly string[] = ["anthropic.com", "moonshot.ai", "openai.com"];

const CANONICAL_AGENT_TRAILERS: readonly string[] = [
  "Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>",
  "Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>",
  "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>",
  "Co-Authored-By: Kimi k3 (kimi-code) <noreply@moonshot.ai>",
  "Co-Authored-By: OpenAI Codex GPT-5.5 <noreply@openai.com>",
  "Co-Authored-By: OpenAI Codex GPT-5.6 Sol <noreply@openai.com>",
];

// A `Co-authored-by:` trailer of any key casing, capturing the email so the host
// can classify the co-author as agent vs human.
const COAUTHOR_LINE = /^co-authored-by:[ \t]*.+?[ \t]*<(?<email>[^>]+)>[ \t]*$/i;

// Custom rule: enforce that every agent `Co-Authored-By` trailer is canonical.
const coAuthoredByCanonical: Rule = (parsed) => {
  const raw = parsed.raw ?? [parsed.header, parsed.body, parsed.footer].filter(Boolean).join("\n");
  const offenders: string[] = [];
  for (const rawLine of raw.split("\n")) {
    const line = rawLine.trimEnd();
    const match = COAUTHOR_LINE.exec(line);
    if (match === null) continue;
    const email = match.groups?.email ?? "";
    const host = email.slice(email.lastIndexOf("@") + 1).toLowerCase();
    const isAgent = AGENT_EMAIL_HOSTS.some((agentHost) => host === agentHost || host.endsWith(`.${agentHost}`));
    if (!isAgent) continue;
    if (!CANONICAL_AGENT_TRAILERS.includes(line)) offenders.push(line);
  }
  if (offenders.length === 0) return [true, ""];
  const offending = offenders.map((line) => `  ✗ ${line}`).join("\n");
  const allowed = CANONICAL_AGENT_TRAILERS.map((trailer) => `  • ${trailer}`).join("\n");
  return [
    false,
    `agent Co-Authored-By trailer must match a canonical registry entry exactly:\n${offending}\nallowed:\n${allowed}`,
  ];
};

// A session-link trailer (`Claude-Session:` and casing/provider variants) is
// agent-harness forensics — contributor-concern, never project-concern — so it
// must never reach the publishable history (core/PUBLISHABLE-HISTORY.md).
const SESSION_TRAILER_LINE = /^(?:claude|gpt|codex)-session:/i;

// Custom rule: reject any session-link trailer.
const noSessionTrailer: Rule = (parsed) => {
  const raw = parsed.raw ?? [parsed.header, parsed.body, parsed.footer].filter(Boolean).join("\n");
  const offenders = raw.split("\n").filter((rawLine) => SESSION_TRAILER_LINE.test(rawLine.trim()));
  if (offenders.length === 0) return [true, ""];
  const offending = offenders.map((line) => `  ✗ ${line.trim()}`).join("\n");
  return [false, `session-link trailers are contributor-concern and must not appear in commit messages:\n${offending}`];
};

const agenticDevPlugin: Plugin = {
  rules: {
    "co-authored-by-canonical": coAuthoredByCanonical,
    "no-session-trailer": noSessionTrailer,
  },
};

// Scopes for the surfaces the core itself introduces into every consumer, so
// the base ships them and a consumer need not restate them:
//   • `core`    — the vendored `.agents/core` submodule pin (ADR 2);
//                 every consumer bumps it via the `core:update` task.
//   • `hazards` — the per-consumer `docs/HAZARDS.md` catalogue the core
//                 prescribes (core/HAZARDS.md; each consumer keeps its own).
// Additive by design: unioned with the consumer's vocabulary, deduped — a
// consumer that still lists one of these is accepted, not rejected. A
// project-specific surface (e.g. a corpus manifest) stays in the consumer's
// own list until it graduates into the core.
const CORE_SCOPES: readonly string[] = [
  "adr",
  "agents",
  "beads",
  "changelog",
  "ci",
  "config",
  "contributing",
  "core",
  "docs",
  "format",
  "gate",
  "git",
  "hazards",
  "readme",
  "repo",
  "scripts",
  "skills",
  "workflow",
];

// Build the consumer config over the shared base. `scopes` is the consumer's
// project-specific scope vocabulary (compound scopes are comma-delimited and
// each part must be a member); it is unioned with `CORE_SCOPES`. New areas are
// added deliberately, never by free-form invention.
export function makeConfig(scopes: readonly string[]): UserConfig {
  const allScopes: string[] = [...new Set([...CORE_SCOPES, ...scopes])];
  return {
    extends: ["@commitlint/config-conventional"],
    plugins: [agenticDevPlugin],
    rules: {
      // Header: a single standalone subject line, ≤ 100 chars, trimmed.
      "header-max-length": [RuleConfigSeverity.Error, "always", 100],
      "header-trim": [RuleConfigSeverity.Error, "always"],

      // Subject: required, no trailing period. `subject-case` is INHERITED from
      // config-conventional, whose `never [sentence-case, start-case,
      // pascal-case, upper-case]` set rejects a capitalized FIRST letter while
      // leaving INTERNAL capitals alone — do NOT add a `lower-case` override
      // (far stricter: it rejects ANY internal capital, mangling proper nouns
      // and acronyms mid-subject).
      "subject-empty": [RuleConfigSeverity.Error, "never"],
      "subject-full-stop": [RuleConfigSeverity.Error, "never", "."],

      // Body & footer: 100-char wrap, with the blank line that keeps the
      // subject standalone promoted from warning to error.
      "body-leading-blank": [RuleConfigSeverity.Error, "always"],
      "body-max-line-length": [RuleConfigSeverity.Error, "always", 100],
      "footer-leading-blank": [RuleConfigSeverity.Error, "always"],

      // Type: a required, lower-case conventional type (config-conventional enum).
      "type-empty": [RuleConfigSeverity.Error, "never"],

      // Scope: required, lower-case, from the fixed vocabulary (core-prescribed
      // scopes unioned with the consumer's own).
      "scope-empty": [RuleConfigSeverity.Error, "never"],
      "scope-case": [RuleConfigSeverity.Error, "always", "lower-case"],
      "scope-enum": [RuleConfigSeverity.Error, "always", { scopes: allScopes, delimiters: [","] }],

      // Attribution: machine co-authors must use a canonical trailer.
      "co-authored-by-canonical": [RuleConfigSeverity.Error, "always"],

      // Hygiene: no session-link forensics trailers in the history.
      "no-session-trailer": [RuleConfigSeverity.Error, "always"],
    },
  };
}

// ── gandr consumer config ────────────────────────────────────────────────────
// NOTE (inlined agentic-dev base): everything above this section is the
// agentic-dev core base fragment (normally imported from
// `./.agents/core/fragments/commitlintrc.base.mts`), inlined here because the
// vendored `.agents/core` submodule is not wired into the reboot yet. When the
// core is re-vendored, restore the import and keep only this consumer section —
// and reconcile the closed-vocabulary override below (gandr deliberately
// narrows the base's additive CORE_SCOPES union; upstream the narrowing).

// Fixed scope vocabulary — CLOSED and deliberately small (owner, 2026-07-21).
// Scopes are broad areas, never per-crate labels: the seven crate-category
// prefixes of the naming schema (docs/workflow/rust.md §conventions) plus four
// documentation/config areas. The subject line and the diff carry crate-level
// specificity; compound scopes are comma-delimited and each part must be a
// member.
//
// Do NOT add scopes without OWNER AUTHORIZATION FIRST — a small fixed set is
// what makes scopes useful; per-surface growth is the failure mode this list
// replaced (57 effective scopes, one per crate, pruned 2026-07-21).
const GANDR_SCOPES: readonly string[] = [
  // crate categories (crates/<category>-*)
  "accel",
  "core",
  "kernel",
  "runtime",
  "storage",
  "surface",
  "theory",
  // also covers docs/workflow process docs — both are how-we-work machinery
  "workflow",
  // documentation / config areas
  // docs/research records
  "analysis",
  // guidance + documentation not otherwise homed (AGENTS, README, KNOWLEDGE, …)
  "docs",
  // workspace config, hooks, mise, treefmt, commitlint, CI, scripts, wt, fuzz
  "repo",
  // the design corpus under docs/gandr/spec
  "spec",
];

// The base's makeConfig unions CORE_SCOPES additively; gandr's vocabulary is
// CLOSED, so the scope-enum rule is overridden to exactly GANDR_SCOPES.
const base = makeConfig(GANDR_SCOPES);
export default {
  ...base,
  rules: {
    ...base.rules,
    "scope-enum": [RuleConfigSeverity.Error, "always", { scopes: [...GANDR_SCOPES], delimiters: [","] }],
  },
} satisfies UserConfig;
