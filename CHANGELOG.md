<!-- SPDX-FileCopyrightText: 2026 Sebastien Rousseau -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Changelog

## Unreleased

### Added

- Diagnostics from the `agtmls-spec` rule set, with rule ids and links.
- Frontmatter completion; `allowed-tools` values state the capability each
  tool implies, because granting `Bash` is a decision and not a typo.
- Code actions: remove invisible characters, narrow `allowed-tools`, create
  `metadata.json`. No action widens a safety policy.
- Document symbols.
- An unarmed server publishes a warning rather than an empty diagnostic list.
  An empty list means clean, and unchecked is not clean.
- Twelve handler tests, including that the capability fix never keeps an
  escalating grant.
