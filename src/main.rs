// SPDX-FileCopyrightText: 2026 Sebastien Rousseau
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `agtmls-lsp` — Language Server Protocol server for `AgtMLS` agent skills.
//!
//! JSON-RPC 2.0 with `Content-Length` framing over stdio. This binary is the
//! transport shim around [`agtmls_lsp::Server`]; all handler logic lives in
//! the library so `cargo test` covers it without standing up an editor.

#![forbid(unsafe_code)]

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use agtmls_lsp::Server;
use agtmls_lsp::framing::{read_message, write_message};
use serde_json::{Value, json};

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

/// Serve messages until end of input, a write failure, or `shutdown`.
///
/// A frame the reader refuses is answered with a JSON-RPC parse error
/// (`-32700`, `id: null`) and the session carries on: one bad frame from a
/// buggy client is not worth killing the editor's language server over.
fn serve(server: &mut Server, reader: &mut impl BufRead, stdout: &mut impl Write) {
    loop {
        let payload = match read_message(reader) {
            Ok(Some(payload)) => payload,
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                eprintln!("agtmls-lsp: refused a frame: {error}");
                let reply = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("parse error: {error}") },
                });
                if write_message(stdout, &reply.to_string()).is_err() {
                    return;
                }
                continue;
            }
            Ok(None) | Err(_) => return,
        };
        let Ok(message) = serde_json::from_str::<Value>(&payload) else {
            continue; // a malformed frame is not worth killing the session over
        };
        if let Some(reply) = server.handle(&message)
            && write_message(stdout, &reply.to_string()).is_err()
        {
            return;
        }
        if server.shutting_down {
            return;
        }
    }
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
    serve(&mut server, &mut stdin.lock(), &mut io::stdout());
    ExitCode::SUCCESS
}
