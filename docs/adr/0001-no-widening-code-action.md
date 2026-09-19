<!-- SPDX-FileCopyrightText: 2026 Sebastien Rousseau -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# ADR 0001 — No code action widens a safety policy

**Status:** accepted · **Date:** 2026-09-19

## Context

`AGT-CAP-001` fires when frontmatter grants a tool the skill's own
`safety_policy` denies — `allowed-tools: Bash` beside
`executes_commands: false`.

There are two ways to make the diagnostic go away. Remove `Bash` from the
grant, or set `executes_commands: true`. Both resolve it. An editor that
offers quick fixes would naturally offer both, and the second is usually the
one an author wants in the moment, because they added `Bash` on purpose.

## Decision

Only the narrowing action is offered. There is no action that edits
`safety_policy`, and a test asserts that no offered action ever does.

## Consequences

**Good.** Granting a skill the ability to execute commands stays a decision
someone makes by typing, in a file they are looking at. The declared policy is
what `allowed_tools` is derived from and what the published index reports, so
widening it is not a local fix — it changes what every consumer is told about
the skill.

**Costly.** The author whose skill genuinely needs `Bash` gets a diagnostic
with a quick fix that does the opposite of what they want, and has to edit
`metadata.json` by hand. That is friction applied to the correct case in order
to remove a one-keystroke path to the incorrect one, and it is the right trade
only because the two are indistinguishable to the editor.

**Mitigation.** Completion on `allowed-tools` states the capability each tool
implies, so the decision is informed before the diagnostic ever appears.

## Alternatives rejected

**Offer both, mark narrowing preferred.** `isPreferred` affects ordering and
the default for "fix all"; it does not stop a single keystroke choosing the
other. The distance between the two must be larger than a cursor position.
