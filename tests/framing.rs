// SPDX-FileCopyrightText: 2026 Sebastien Rousseau
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Framing tests: what the reader does with frames a client should never
//! send but a buggy or hostile one can.

use std::io::{self, BufReader, Cursor};

use agtmls_lsp::framing::read_message;

fn reader(bytes: &[u8]) -> BufReader<Cursor<Vec<u8>>> {
    BufReader::new(Cursor::new(bytes.to_vec()))
}

fn assert_refused(result: &io::Result<Option<String>>) {
    match result {
        Err(error) => assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{error}"),
        Ok(other) => panic!("expected a refused frame, got {other:?}"),
    }
}

#[test]
fn reads_a_well_formed_frame() {
    let mut input = reader(b"Content-Length: 2\r\n\r\n{}");
    assert_eq!(read_message(&mut input).unwrap().as_deref(), Some("{}"));
    assert_eq!(read_message(&mut input).unwrap(), None);
}

#[test]
fn header_name_is_case_insensitive() {
    let mut input = reader(b"content-length: 2\r\n\r\n{}");
    assert_eq!(read_message(&mut input).unwrap().as_deref(), Some("{}"));
}

#[test]
fn maximal_length_is_refused_without_allocating() {
    let mut input = reader(b"Content-Length: 18446744073709551615\r\n\r\n{}");
    assert_refused(&read_message(&mut input));
}

#[test]
fn length_over_the_cap_is_refused_and_the_stream_resyncs() {
    let body = vec![b' '; 16 * 1024 * 1024 + 1];
    let mut bytes = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    bytes.extend_from_slice(&body);
    bytes.extend_from_slice(b"Content-Length: 2\r\n\r\n{}");
    let mut input = reader(&bytes);
    assert_refused(&read_message(&mut input));
    assert_eq!(read_message(&mut input).unwrap().as_deref(), Some("{}"));
}

#[test]
fn non_numeric_length_is_refused() {
    let mut input = reader(b"Content-Length: twelve\r\n\r\n");
    assert_refused(&read_message(&mut input));
}

#[test]
fn missing_length_is_refused() {
    let mut input = reader(b"Content-Type: application/vscode-jsonrpc\r\n\r\n");
    assert_refused(&read_message(&mut input));
}

#[test]
fn over_long_header_line_is_refused_and_the_stream_resyncs() {
    let mut bytes = b"X-Padding: ".to_vec();
    bytes.extend(std::iter::repeat_n(b'a', 64 * 1024));
    bytes.extend_from_slice(b"\r\n\r\nContent-Length: 2\r\n\r\n{}");
    let mut input = reader(&bytes);
    assert_refused(&read_message(&mut input));
    assert_eq!(read_message(&mut input).unwrap().as_deref(), Some("{}"));
}

#[test]
fn invalid_utf8_body_is_refused_and_the_stream_resyncs() {
    let mut input = reader(b"Content-Length: 2\r\n\r\n\xff\xfeContent-Length: 2\r\n\r\n{}");
    assert_refused(&read_message(&mut input));
    assert_eq!(read_message(&mut input).unwrap().as_deref(), Some("{}"));
}

#[test]
fn truncated_body_is_an_error() {
    let mut input = reader(b"Content-Length: 10\r\n\r\n{}");
    assert!(read_message(&mut input).is_err());
}

#[test]
fn server_answers_a_refused_frame_and_keeps_serving() {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let mut input = b"content-length: x\r\n\r\n".to_vec();
    input.extend_from_slice(
        format!("Content-Length: {}\r\n\r\n{request}", request.len()).as_bytes(),
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_agtmls-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&input).unwrap();
    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    assert!(stdout.contains(r#""code":-32700"#), "{stdout}");
    assert!(
        stdout.contains(r#""id":1"#),
        "no reply after the bad frame: {stdout}"
    );
}
