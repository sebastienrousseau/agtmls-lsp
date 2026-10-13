<!-- SPDX-FileCopyrightText: 2026 Sebastien Rousseau -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Changelog

## [0.0.3] - Unreleased

### Changed

- **The MSRV is now Rust 1.89.0** (was 1.86.0), the floor shared by the
  portfolio's Rust repositories. `rust-version` and the README badge move
  together. Three nested `if let` blocks became let chains, which clippy
  asks for once the floor is past 1.88.

## [0.0.2] - 2026-10-11

### Added

- Diagnostics from the `agtmls-spec` rule set, with rule ids and links.
- Frontmatter completion; `allowed-tools` values state the capability each
  tool implies, because granting `Bash` is a decision and not a typo.
- Code actions: remove invisible characters, narrow `allowed-tools`, create
  `metadata.json`. No action widens a safety policy.
- Document symbols.
- A VS Code client in `editors/vscode`, deliberately thin.
- An unarmed server publishes a warning rather than an empty diagnostic list.
  An empty list means clean, and unchecked is not clean.
- Twelve handler tests, including that the capability fix never keeps an
  escalating grant.
