<!-- SPDX-FileCopyrightText: 2026 Sebastien Rousseau -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# Changelog

## [0.0.3] - 2026-11-07

### Changed

- **`agtmls-core` comes from crates.io**, pinned exactly at `=0.0.3`
  (was a git revision), so this crate can be published and a consumer can
  audit the dependency from the registry. Structural skill checks now run
  against the loaded rule set, as `agtmls-core` 0.0.3 requires; a server
  without one still reports that the file is unchecked.
- `KEYS.asc` trusts a second maintainer signing key,
  `SHA256:kIOPAavp1TCEauTr1tTIN3cv+tSs6F9m/4lZjuM9tqk`, so release tags can
  be signed on either maintainer machine. Each key line now carries its
  fingerprint for out-of-band checks.
- **The MSRV is now Rust 1.89.0** (was 1.86.0), the floor shared by the
  portfolio's Rust repositories. `rust-version` and the README badge move
  together. Three nested `if let` blocks became let chains, which clippy
  asks for once the floor is past 1.88.

### Fixed

- **Message framing no longer trusts the client for an allocation size.**
  A `Content-Length` of `18446744073709551615` used to panic the server
  (`capacity overflow`), and any large value allocated that many bytes. A
  body over 16 MiB is now skipped unbuffered and refused, and a header line
  over 8 KiB is refused. A missing or non-numeric `Content-Length` is
  refused rather than read as zero, a body that is not UTF-8 is refused
  rather than decoded lossily, and the header name is matched
  case-insensitively as the base protocol requires. Each refused frame is
  answered with a JSON-RPC parse error (`-32700`) and the session goes on.

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
