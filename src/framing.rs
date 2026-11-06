// SPDX-FileCopyrightText: 2026 Sebastien Rousseau
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `Content-Length` framing for the LSP base protocol.
//!
//! Lives in the library rather than `main.rs` so it can be tested against an
//! in-memory reader.
//!
//! The reader never trusts the peer for an allocation size: a header line is
//! read into at most [`MAX_HEADER_LINE`] bytes and a body into at most
//! [`MAX_BODY`]. A frame that breaks a rule is refused with an
//! [`io::ErrorKind::InvalidData`] error, after its bytes have been consumed
//! where that is possible, so the caller can answer it and read the next one.

use std::io::{self, BufRead, Read, Write};

/// Largest message body accepted, in bytes.
pub const MAX_BODY: usize = 16 * 1024 * 1024;

/// Largest single header line accepted, in bytes, including its line ending.
pub const MAX_HEADER_LINE: usize = 8 * 1024;

/// The header block of one frame, as far as it could be understood.
#[derive(Default)]
struct Header {
    /// The declared body length, once a valid `Content-Length` is seen.
    length: Option<usize>,
    /// Why the frame must be refused, if it must.
    refusal: Option<String>,
}

/// Read one `Content-Length`-framed message.
///
/// Returns `Ok(None)` at end of input. Header names are matched
/// case-insensitively, as the base protocol requires; headers other than
/// `Content-Length` are ignored.
///
/// # Errors
///
/// An error of kind [`io::ErrorKind::InvalidData`] refuses one frame and
/// leaves the reader at the start of the next one wherever the frame's
/// extent is known: a header line over [`MAX_HEADER_LINE`], a missing or
/// unparseable `Content-Length`, a length over [`MAX_BODY`] (whose body is
/// skipped without being buffered), or a body that is not UTF-8. Any other
/// error, including end of input inside a body, comes from the reader and
/// means the stream is unusable.
pub fn read_message(reader: &mut impl BufRead) -> io::Result<Option<String>> {
    let Some(header) = read_header(reader)? else {
        return Ok(None); // clean EOF: the editor closed the pipe
    };
    match (header.length, header.refusal) {
        (Some(length), None) if length <= MAX_BODY => read_body(reader, length).map(Some),
        (Some(length), refusal) => {
            discard(reader, length)?;
            let why = refusal.unwrap_or_else(|| {
                format!("Content-Length {length} exceeds the {MAX_BODY} byte limit")
            });
            Err(invalid(why))
        }
        (None, refusal) => {
            Err(invalid(refusal.unwrap_or_else(|| {
                "missing Content-Length header".to_owned()
            })))
        }
    }
}

/// Read header lines up to the blank line that ends them, or `None` at EOF.
fn read_header(reader: &mut impl BufRead) -> io::Result<Option<Header>> {
    let mut header = Header::default();
    let mut line = Vec::with_capacity(128);
    loop {
        line.clear();
        let limit = (MAX_HEADER_LINE + 1) as u64;
        if reader.by_ref().take(limit).read_until(b'\n', &mut line)? == 0 {
            return Ok(None);
        }
        if line.len() > MAX_HEADER_LINE {
            reader.skip_until(b'\n')?;
            header
                .refusal
                .get_or_insert_with(|| format!("header line over {MAX_HEADER_LINE} bytes"));
            continue;
        }
        let text = line.trim_ascii_end();
        if text.is_empty() {
            return Ok(Some(header));
        }
        parse_header_line(text, &mut header);
    }
}

/// Record what one header line says about the frame.
fn parse_header_line(line: &[u8], header: &mut Header) {
    let Some(colon) = line.iter().position(|&byte| byte == b':') else {
        return;
    };
    let (name, value) = (&line[..colon], &line[colon + 1..]);
    if !name.trim_ascii().eq_ignore_ascii_case(b"content-length") {
        return;
    }
    let parsed = std::str::from_utf8(value.trim_ascii())
        .ok()
        .and_then(|digits| digits.parse::<usize>().ok());
    header.length = parsed;
    if parsed.is_none() {
        header
            .refusal
            .get_or_insert_with(|| "Content-Length is not a valid length".to_owned());
    }
}

fn read_body(reader: &mut impl BufRead, length: usize) -> io::Result<String> {
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;
    String::from_utf8(body).map_err(|_| invalid("message body is not valid UTF-8".to_owned()))
}

/// Consume `length` body bytes without buffering them.
fn discard(reader: &mut impl BufRead, length: usize) -> io::Result<()> {
    let length = u64::try_from(length).unwrap_or(u64::MAX);
    io::copy(&mut reader.by_ref().take(length), &mut io::sink())?;
    Ok(())
}

fn invalid(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why)
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
