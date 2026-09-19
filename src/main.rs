// SPDX-FileCopyrightText: 2026 Sebastien Rousseau
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `agtmls-lsp` — Language Server Protocol server for `AgtMLS` agent skills.
//!
//! JSON-RPC 2.0 with `Content-Length` framing over stdio. This binary is the
//! transport shim around [`agtmls_lsp::Server`]; all handler logic lives in
//! the library so `cargo test` covers it without standing up an editor.

#![forbid(unsafe_code)]

// BufRead: Read, so read_exact is already in scope.
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use agtmls_lsp::Server;

const HELP: &str = "\
agtmls-lsp — Language Server Protocol server for AgtMLS agent skills.

USAGE:
  agtmls-lsp                 Start the LSP stdio server.
  agtmls-lsp --version | -V  Print version and exit.
  agtmls-lsp --help | -h     Print this help and exit.

ENVIRONMENT:
  AGTMLS_SPEC   Checkout of https://github.com/sebastienrousseau/agtmls-spec.
                Without it the server runs but publishes a warning rather than
                an empty diagnostic list: unchecked is not the same as clean.

It speaks Content-Length-framed JSON-RPC over stdio and is not meant to be
used interactively. Configure your editor to spawn it.";

/// Read one `Content-Length`-framed message.
fn read_message(stdin: &mut impl BufRead) -> io::Result<Option<String>> {
    let mut length = 0usize;
    loop {
        let mut header = String::new();
        if stdin.read_line(&mut header)? == 0 {
            return Ok(None); // clean EOF: the editor closed the pipe
        }
        let header = header.trim_end();
        if header.is_empty() {
            break; // blank line ends the headers
        }
        if let Some(value) = header.strip_prefix("Content-Length:") {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    if length == 0 {
        return Ok(Some(String::new()));
    }
    let mut buffer = vec![0u8; length];
    stdin.read_exact(&mut buffer)?;
    Ok(Some(String::from_utf8_lossy(&buffer).into_owned()))
}

fn write_message(stdout: &mut impl Write, payload: &str) -> io::Result<()> {
    write!(stdout, "Content-Length: {}\r\n\r\n{payload}", payload.len())?;
    stdout.flush()
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("agtmls-lsp {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    let mut server = Server::new();
    if !server.has_rules() {
        eprintln!(
            "agtmls-lsp: no rule set; set AGTMLS_SPEC. Diagnostics will say so rather than \
             reporting files as clean."
        );
    }

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = io::stdout();

    while let Ok(Some(payload)) = read_message(&mut reader) {
        if payload.is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<serde_json::Value>(&payload) else {
            continue; // a malformed frame is not worth killing the session over
        };
        if let Some(reply) = server.handle(&message) {
            if write_message(&mut stdout, &reply.to_string()).is_err() {
                break;
            }
        }
        if server.shutting_down {
            break;
        }
    }
    ExitCode::SUCCESS
}
