<!-- SPDX-FileCopyrightText: 2026 Sebastien Rousseau -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# AgtMLS for VS Code

Live security diagnostics for agent skills, from
[`agtmls-lsp`](https://github.com/sebastienrousseau/agtmls-lsp).

## Requirements

```bash
cargo install agtmls-lsp
```

Then set `agtmls.specPath` to a checkout of
[`agtmls-spec`](https://github.com/sebastienrousseau/agtmls-spec).

Without it the server still runs, and says so: it publishes a warning rather
than an empty diagnostic list. An empty list means *clean*, and unchecked is
not clean — which is the one wrong answer a security tool must never give.

## Settings

| Setting | Default | Meaning |
| :--- | :--- | :--- |
| `agtmls.serverPath` | `agtmls-lsp` | Path to the server binary |
| `agtmls.specPath` | *(unset)* | Path to an `agtmls-spec` checkout |

## Why the client is thin

Every rule, code action and severity decision lives in the server, which is
the same engine the CLI runs. An editor that reimplements any of it will
eventually disagree with CI — and the author will believe the editor.
