// SPDX-FileCopyrightText: 2026 Sebastien Rousseau
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `Content-Length` framing for the LSP base protocol.
//!
//! Lives in the library rather than `main.rs` so it can be tested against an
//! in-memory reader.

use std::io::{self, BufRead, Write};

/// Read one `Content-Length`-framed message.
///
/// # Errors
///
/// Returns any I/O error from `stdin`.
pub fn read_message(stdin: &mut impl BufRead) -> io::Result<Option<String>> {
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

/// Write one `Content-Length`-framed message and flush.
///
/// # Errors
///
/// Returns any I/O error from `stdout`.
pub fn write_message(stdout: &mut impl Write, payload: &str) -> io::Result<()> {
    write!(stdout, "Content-Length: {}\r\n\r\n{payload}", payload.len())?;
    stdout.flush()
}
