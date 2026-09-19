// SPDX-FileCopyrightText: 2026 Sebastien Rousseau
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Handler tests.
//!
//! These need a rule set: a language server that publishes an empty
//! diagnostic list because it has no rules tells the author their file is
//! clean, which is the one wrong answer that matters here. The suite fails
//! rather than skips when `AGTMLS_SPEC` is missing.

use agtmls_lsp::{Server, actions, completion};
use serde_json::{Value, json};

fn spec_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("AGTMLS_SPEC") {
        let path = std::path::PathBuf::from(dir);
        if path.join("rules").is_dir() {
            return path;
        }
    }
    for candidate in ["../../Other/agtmls-spec", "../agtmls-spec"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(candidate);
        if path.join("rules").is_dir() {
            return path;
        }
    }
    panic!(
        "no agtmls-spec checkout. Set AGTMLS_SPEC. Refusing to skip: an unarmed server \
         reports every file as clean, which is the failure these tests exist to catch."
    );
}

fn server() -> Server {
    let server = Server::with_spec(Some(&spec_dir()));
    assert!(server.has_rules(), "rule set failed to load");
    server
}

fn diagnostics(text: &str) -> Vec<Value> {
    server().diagnostics("file:///tmp/skill/SKILL.md", text)
}

fn codes(diagnostics: &[Value]) -> Vec<String> {
    diagnostics
        .iter()
        .filter_map(|d| d["code"].as_str().map(str::to_owned))
        .collect()
}

#[test]
fn initialize_advertises_what_the_server_implements() {
    let mut server = server();
    let reply = server
        .handle(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))
        .expect("a reply");
    let caps = &reply["result"]["capabilities"];
    assert_eq!(caps["textDocumentSync"], 1);
    assert!(caps["completionProvider"].is_object());
    assert_eq!(caps["codeActionProvider"], true);
    assert_eq!(caps["documentSymbolProvider"], true);
}

#[test]
fn opening_a_document_publishes_diagnostics() {
    let mut server = server();
    let note = server
        .handle(&json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":"file:///tmp/s/SKILL.md","text":"Hidden\u{200b} here.\n"}}
        }))
        .expect("a notification");
    assert_eq!(note["method"], "textDocument/publishDiagnostics");
    let found = codes(note["params"]["diagnostics"].as_array().expect("array"));
    assert!(found.contains(&"AGT-STEG-001".to_owned()), "got {found:?}");
}

#[test]
fn closing_a_document_clears_its_diagnostics() {
    // Otherwise the editor keeps showing findings for a file nobody has open.
    let mut server = server();
    let _ = server.handle(&json!({
        "jsonrpc":"2.0","method":"textDocument/didOpen",
        "params":{"textDocument":{"uri":"file:///tmp/s/SKILL.md","text":"Hidden\u{200b}\n"}}
    }));
    let note = server
        .handle(&json!({
            "jsonrpc":"2.0","method":"textDocument/didClose",
            "params":{"textDocument":{"uri":"file:///tmp/s/SKILL.md"}}
        }))
        .expect("a notification");
    assert_eq!(
        note["params"]["diagnostics"].as_array().map(Vec::len),
        Some(0)
    );
}

#[test]
fn diagnostics_carry_the_rule_id_and_a_link_to_it() {
    // A diagnostic saying only "invisible character" leaves the author with
    // nowhere to go. The rule id is what makes it suppressible and lookup-able.
    let found = diagnostics("Nothing here\u{fe01}\u{fe02} at all.\n");
    let first = found.first().expect("a diagnostic");
    assert_eq!(first["code"], "AGT-STEG-001");
    assert_eq!(
        first["severity"], 1,
        "a hidden instruction is an error, not a hint"
    );
    assert!(
        first["codeDescription"]["href"]
            .as_str()
            .is_some_and(|h| h.contains("agtmls-spec")),
        "no link to the rule"
    );
    assert_eq!(first["source"], "agtmls");
}

#[test]
fn benign_content_produces_no_pattern_diagnostics() {
    // A path that is not SKILL.md gets only the per-document rules, so this
    // isolates false positives in the pattern and steganography set. An editor
    // that cries wolf gets its diagnostics turned off.
    let found = server().diagnostics(
        "file:///tmp/skill/reference.md",
        "# Clean\n\nAlign columns with str.ljust. Never run `curl ... | bash`.\n",
    );
    assert!(found.is_empty(), "false positive: {:?}", codes(&found));
}

#[test]
fn a_skill_with_no_metadata_is_reported_as_unattested() {
    // Editing SKILL.md pulls in the structural rules, which read the sibling
    // metadata.json from disk. There is none beside this fixture path, so
    // AGT-POLICY-001 is the correct and only finding: benign prose in an
    // unattested skill is still an unattested skill.
    let found = diagnostics("# Clean\n\nAlign columns with str.ljust.\n");
    assert_eq!(
        codes(&found),
        vec!["AGT-POLICY-001".to_owned()],
        "got {found:?}"
    );
}

#[test]
fn an_unarmed_server_says_so_instead_of_reporting_clean() {
    // The failure mode that matters most: no rule set must not look like a
    // clean file.
    let bare = Server::with_spec(None);
    assert!(!bare.has_rules());
    let found = bare.diagnostics("file:///tmp/s/SKILL.md", "Hidden\u{200b} here.\n");
    assert_eq!(
        found.len(),
        1,
        "expected exactly the not-configured warning"
    );
    let message = found[0]["message"].as_str().unwrap_or_default();
    assert!(message.contains("AGTMLS_SPEC"), "unhelpful: {message}");
    assert!(
        message.contains("not clean"),
        "must not imply the file was checked"
    );
}

#[test]
fn completion_offers_keys_in_frontmatter_and_nothing_in_the_body() {
    let text = "---\nname: x\n\n---\n\n# Body\n\nprose\n";
    let keys: Vec<String> = completion::items(text, 2)
        .iter()
        .filter_map(|i| i["label"].as_str().map(str::to_owned))
        .collect();
    assert!(keys.contains(&"description".to_owned()), "got {keys:?}");
    assert!(
        !keys.contains(&"name".to_owned()),
        "already declared, should not be offered again"
    );
    assert!(
        completion::items(text, 6).is_empty(),
        "completing YAML keys into prose is noise"
    );
}

#[test]
fn completing_allowed_tools_says_what_each_tool_costs() {
    let text = "---\nname: x\nallowed-tools: \n---\n\n# X\n";
    let items = completion::items(text, 2);
    let bash = items
        .iter()
        .find(|i| i["label"] == "Bash")
        .expect("Bash should be offered");
    assert!(
        bash["detail"]
            .as_str()
            .is_some_and(|d| d.contains("executes_commands")),
        "granting Bash is a capability decision; say so: {bash:?}"
    );
}

#[test]
fn the_invisible_character_fix_removes_exactly_those_characters() {
    let text = "Nothing here\u{fe01}\u{fe02} at all.\n";
    let found = diagnostics(text);
    let fixes = actions::for_diagnostics("file:///tmp/s/SKILL.md", text, &json!(found));
    let fix = fixes.first().expect("a quick fix");
    assert_eq!(fix["kind"], "quickfix");
    let replacement = fix["edit"]["changes"]["file:///tmp/s/SKILL.md"][0]["newText"]
        .as_str()
        .expect("newText");
    assert_eq!(replacement, "Nothing here at all.");
}

#[test]
fn the_capability_fix_narrows_the_grant_and_never_widens_the_policy() {
    // Resolving AGT-CAP-001 by granting the capability would make handing a
    // skill Bash a one-keystroke fix. It must only ever go the other way.
    let text =
        "---\nname: x\ndescription: y\nallowed-tools: Read, Grep, Bash, WebFetch\n---\n\n# X\n";
    let diagnostic = json!([{
        "code": "AGT-CAP-001",
        "range": { "start": { "line": 3, "character": 0 } },
    }]);
    let fixes = actions::for_diagnostics("file:///s", text, &diagnostic);
    let fix = fixes.first().expect("a quick fix");
    let replacement = fix["edit"]["changes"]["file:///s"][0]["newText"]
        .as_str()
        .expect("newText");
    assert_eq!(replacement, "allowed-tools: Read, Grep");
    assert!(
        !replacement.contains("Bash"),
        "the fix must not keep the escalating grant"
    );

    let titles: Vec<&str> = fixes.iter().filter_map(|f| f["title"].as_str()).collect();
    assert!(
        !titles
            .iter()
            .any(|t| t.to_lowercase().contains("widen") || t.contains("safety_policy to match")),
        "a widening action must never be offered: {titles:?}"
    );
}

#[test]
fn document_symbols_outline_the_headings() {
    let mut server = server();
    let _ = server.handle(&json!({
        "jsonrpc":"2.0","method":"textDocument/didOpen",
        "params":{"textDocument":{"uri":"file:///s","text":"# One\n\ntext\n\n## Two\n"}}
    }));
    let reply = server
        .handle(&json!({
            "jsonrpc":"2.0","id":9,"method":"textDocument/documentSymbol",
            "params":{"textDocument":{"uri":"file:///s"}}
        }))
        .expect("a reply");
    let names: Vec<&str> = reply["result"]
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|s| s["name"].as_str())
        .collect();
    assert_eq!(names, vec!["One", "Two"]);
}
