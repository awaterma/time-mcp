---
id: 2026-06-24-accepted-rust-audit-patterns
date: 2026-06-24
status: accepted
tags: [rust, audit, false-positives, idioms]
relates_to:
  - audit-manual-err-return
  - audit-rust-let-binding-count-high
  - audit-rust-let-mut-count-high
---

# Accepted Rust patterns flagged by audit-only heuristics

## Context

A full `audit_codebase` sweep of time-mcp surfaced 14 warn-level hits
across three audit-phase rules. Each was inspected against the actual
code (not the line-1 file-level marker). All hits are idiomatic Rust,
not debt. The three rules use substring / built-in-count predicates that
cannot be made precise: there is no negation predicate to exclude a
match's fallback arm, and the `let`-count predicates expose no threshold
argument. So precision-tuning the rules is not possible — the choice was
remove (lose future signal) vs. document the patterns as accepted. We
chose to document.

## Decision

The following patterns are reviewed and accepted in this codebase. The
rules are kept (they may still catch genuine misuse later), but these
specific shapes should not be "fixed":

1. **`=> return Err(...)` as a match fallback arm**
   (`audit-manual-err-return`). In a value-producing `match` assigned to
   a `let`, the `_ => return Err(...)` arm constructs the error inline;
   there is no `Result` to propagate, so `?` cannot replace it. Sites:
   `config.rs:44`, `handlers/http.rs:146`, `tools.rs:53,159,206`.

2. **High outer-scope `let` binding count**
   (`audit-rust-let-binding-count-high`). The tool-handler functions in
   `tools.rs` and the test functions legitimately bind several
   intermediates; they are coherent and readable as written.

3. **High `let mut` count in IO loops**
   (`audit-rust-let-mut-count-high`). `handlers/stdio.rs::run` needs
   `let mut stdout`, `let mut reader`, and `let mut line` for the async
   stdin read loop — every `mut` is required.

Separately, two real issues found in the same sweep were fixed (not
deferred): the two block-level `.unwrap()`s in `tools.rs` (Jan-1
reference-date construction → `.ok_or_else(...)?`), and a stale test
assertion (`protocolVersion == "1.0"` → `"2025-06-18"`, matching
`DEFAULT_PROTOCOL_VERSION`).

## Enforcement

No automated rule change. The three audit rules remain in
`rules.json` as informational nudges; they are `phase: audit`,
`priority: 3`, and never block hook-time work. This page records the
accepted patterns so future audit sweeps and future sessions do not
re-litigate them.

## Consequences

- Future `audit_codebase` runs will keep reporting these ~14 warnings.
  That is expected; treat this page as the disposition.
- If one of these rules ever fires on a *genuinely* replaceable
  `return Err`, a god-function, or an unnecessary `mut`, that hit is
  still actionable — this page only blesses the shapes enumerated above.
- The two code fixes are covered by the existing test suite
  (`test_get_timezone_info_with_dst`, `test_http_get_capabilities`);
  all 56 tests pass.
