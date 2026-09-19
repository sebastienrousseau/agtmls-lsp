<!-- SPDX-FileCopyrightText: 2026 Sebastien Rousseau -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

<h1 align="center">agtmls-lsp</h1>

<p align="center">
  Live security diagnostics for agent skills, in your editor.
</p>

---

## Why

Writing a skill today has no editor support at all. You write YAML frontmatter
by hand, guess at `allowed-tools`, and find out at CI time — or, for the rules
that matter most, at the point where a model follows an instruction you could
not see, because it was written in variation selectors.

This puts the analyzer where the author is. The diagnostics are the same rules
the CLI enforces, loaded from
[`agtmls-spec`](https://github.com/sebastienrousseau/agtmls-spec), because a
diagnostic that disagrees with CI is worse than none: the author fixes what the
editor shows and the build fails anyway.

## Install

```bash
cargo install agtmls-lsp
```

```lua
-- Neovim
require('lspconfig.configs').agtmls = {
  default_config = {
    cmd = { 'agtmls-lsp' },
    filetypes = { 'markdown' },
    root_dir = require('lspconfig.util').root_pattern('SKILL.md', '.git'),
  },
}
require('lspconfig').agtmls.setup {}
```

Set `AGTMLS_SPEC` to a checkout of the specification. Without it the server
still runs — completion and code actions remain useful — but it publishes a
warning rather than an empty diagnostic list. **Unchecked is not the same as
clean**, and an empty list says clean.

## What you get

| Capability | Behaviour |
| :--- | :--- |
| Diagnostics | Every analyzer rule, live, with the rule id and a link to its definition |
| Completion | Frontmatter keys, and `allowed-tools` values annotated with the capability each implies |
| Code actions | See below |
| Document symbols | Heading outline |

## Code actions resolve findings in the safe direction only

- **Remove invisible characters** — one keystroke on an `AGT-STEG-001`.
- **Narrow `allowed-tools`** to what `safety_policy` permits, resolving
  `AGT-CAP-001` by removing the grant.
- **Create `metadata.json`** from the template, review-gated.

There is deliberately **no action that widens a safety policy to match a
frontmatter grant.** It would resolve the same diagnostic, and it would make
handing a skill `Bash` a one-keystroke fix. An editor that does that has not
helped anyone.

## Design

`Content-Length`-framed JSON-RPC over stdio. All handler logic lives in the
library crate so `cargo test` covers it without standing up an editor; the
binary is the framing shim.

Document sync is full rather than incremental: skills are small, and an
incremental implementation is a source of bugs the size does not justify.

The rule source is a constructor parameter, not ambient environment, so a test
can build an armed server and an unarmed one in the same process — which
matters, because "what does this do with no rules?" is the question worth
asking.

## Licence

Apache-2.0 OR MIT.
