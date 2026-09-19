// SPDX-FileCopyrightText: 2026 Sebastien Rousseau
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Language Server Protocol implementation for `AgtMLS` agent skills.
//!
//! Writing a skill today has no editor support at all: you write YAML
//! frontmatter by hand, guess at `allowed-tools`, and find out at CI time —
//! or, for the rules that matter, at the point where a model follows an
//! instruction you could not see.
//!
//! This puts the analyzer in the editor. The diagnostics are the same rules
//! the CLI enforces, loaded from `agtmls-spec`, because a diagnostic that
//! disagrees with CI is worse than none: the author fixes what the editor
//! shows and the build fails anyway.
//!
//! All handler logic lives here so `cargo test` covers it without standing up
//! an editor; `main.rs` is the `Content-Length` framing shim.

#![forbid(unsafe_code)]

pub mod actions;
pub mod completion;

use std::collections::HashMap;

use agtmls_core::{Analyzer, RuleSet, skill};
use serde_json::{Value, json};

/// Open documents, by URI.
type Documents = HashMap<String, String>;

/// The server state.
pub struct Server {
    analyzer: Option<Analyzer>,
    rules: Option<RuleSet>,
    documents: Documents,
    /// Set once the client has sent `shutdown`.
    pub shutting_down: bool,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

/// A rule set is loaded from `AGTMLS_SPEC`. Without one the server still
/// runs — completion and code actions remain useful — but it reports the
/// absence rather than publishing an empty diagnostic list, which an author
/// would reasonably read as "this file is clean".
impl Server {
    /// Build a server, loading the rule set from `AGTMLS_SPEC` if it is set.
    #[must_use]
    pub fn new() -> Self {
        let spec = std::env::var("AGTMLS_SPEC")
            .ok()
            .map(std::path::PathBuf::from);
        Self::with_spec(spec.as_deref())
    }

    /// Build a server against an explicit `agtmls-spec` checkout.
    ///
    /// The rule source is a parameter rather than ambient environment so a
    /// test can construct an armed server and an unarmed one in the same
    /// process. Mutating the environment to do that is unsafe in edition 2024,
    /// and this crate forbids unsafe -- which was the right nudge.
    #[must_use]
    pub fn with_spec(spec: Option<&std::path::Path>) -> Self {
        let rules = spec
            .filter(|p| p.join("rules").is_dir())
            .and_then(|p| RuleSet::load(&p.join("rules")).ok());
        Self {
            analyzer: rules.clone().map(Analyzer::new),
            rules,
            documents: Documents::new(),
            shutting_down: false,
        }
    }

    /// Whether the analyzer is armed.
    #[must_use]
    pub const fn has_rules(&self) -> bool {
        self.analyzer.is_some()
    }

    /// Handle one LSP message, returning a reply when the protocol wants one.
    #[must_use]
    pub fn handle(&mut self, message: &Value) -> Option<Value> {
        let id = message.get("id").cloned();
        let method = message["method"].as_str()?;
        let params = &message["params"];

        match method {
            "initialize" => Some(Self::reply(id, Self::capabilities())),
            "initialized" => None,
            "shutdown" => {
                self.shutting_down = true;
                Some(Self::reply(id, Value::Null))
            }
            "textDocument/didOpen" => {
                let uri = params["textDocument"]["uri"].as_str()?.to_owned();
                let text = params["textDocument"]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                self.documents.insert(uri.clone(), text);
                Some(self.diagnostics_notification(&uri))
            }
            "textDocument/didChange" => {
                let uri = params["textDocument"]["uri"].as_str()?.to_owned();
                // Full sync: the documents are small and an incremental
                // implementation is a source of bugs the size does not justify.
                let text = params["contentChanges"][0]["text"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                self.documents.insert(uri.clone(), text);
                Some(self.diagnostics_notification(&uri))
            }
            "textDocument/didClose" => {
                let uri = params["textDocument"]["uri"].as_str()?.to_owned();
                self.documents.remove(&uri);
                // Clear the diagnostics, or the editor keeps showing findings
                // for a file that is no longer open.
                Some(json!({
                    "jsonrpc": "2.0",
                    "method": "textDocument/publishDiagnostics",
                    "params": { "uri": uri, "diagnostics": [] },
                }))
            }
            "textDocument/completion" => {
                let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
                let line =
                    usize::try_from(params["position"]["line"].as_u64().unwrap_or(0)).unwrap_or(0);
                let text = self
                    .documents
                    .get(uri)
                    .map(String::as_str)
                    .unwrap_or_default();
                Some(Self::reply(
                    id,
                    json!({ "isIncomplete": false, "items": completion::items(text, line) }),
                ))
            }
            "textDocument/codeAction" => {
                let uri = params["textDocument"]["uri"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                let text = self.documents.get(&uri).cloned().unwrap_or_default();
                Some(Self::reply(
                    id,
                    json!(actions::for_diagnostics(
                        &uri,
                        &text,
                        &params["context"]["diagnostics"]
                    )),
                ))
            }
            "textDocument/documentSymbol" => {
                let uri = params["textDocument"]["uri"].as_str().unwrap_or_default();
                let text = self
                    .documents
                    .get(uri)
                    .map(String::as_str)
                    .unwrap_or_default();
                Some(Self::reply(id, json!(Self::symbols(text))))
            }
            _ => id.map(|id| {
                json!({ "jsonrpc": "2.0", "id": id,
                        "error": { "code": -32601, "message": format!("unhandled: {method}") } })
            }),
        }
    }

    fn reply(id: Option<Value>, result: Value) -> Value {
        // Built by hand rather than with json!, which would borrow `result`
        // and leave it unconsumed.
        let mut envelope = serde_json::Map::with_capacity(3);
        envelope.insert("jsonrpc".to_owned(), Value::from("2.0"));
        envelope.insert("id".to_owned(), id.unwrap_or(Value::Null));
        envelope.insert("result".to_owned(), result);
        Value::Object(envelope)
    }

    fn capabilities() -> Value {
        json!({
            "capabilities": {
                // Full sync: see didChange.
                "textDocumentSync": 1,
                "completionProvider": { "triggerCharacters": [":", " ", ","] },
                "codeActionProvider": true,
                "documentSymbolProvider": true,
            },
            "serverInfo": { "name": "agtmls-lsp", "version": env!("CARGO_PKG_VERSION") },
        })
    }

    /// Findings for a document, as LSP diagnostics.
    #[must_use]
    pub fn diagnostics(&self, uri: &str, text: &str) -> Vec<Value> {
        let Some(analyzer) = &self.analyzer else {
            // Reported once, as a warning on line 1, rather than silently
            // publishing nothing: an empty diagnostic list means "clean".
            return vec![json!({
                "range": { "start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0} },
                "severity": 2,
                "source": "agtmls",
                "message": "No rule set loaded. Set AGTMLS_SPEC to a checkout of \
                            https://github.com/sebastienrousseau/agtmls-spec; until then this \
                            file is unchecked, not clean.",
            })];
        };

        let name = uri.rsplit('/').next().unwrap_or("SKILL.md");
        let mut findings = analyzer.audit_str(name, text);

        // Structural rules need the skill, not the document. Only SKILL.md
        // has siblings worth reading, and reading them from the editor's
        // buffer is not possible, so the on-disk metadata is used.
        if name == "SKILL.md" {
            if let Some(dir) = uri.strip_prefix("file://").map(std::path::PathBuf::from) {
                let mut files = skill::SkillFiles::new();
                files.insert("SKILL.md".to_owned(), text.to_owned());
                if let Some(parent) = dir.parent() {
                    if let Ok(meta) = std::fs::read_to_string(parent.join("metadata.json")) {
                        files.insert("metadata.json".to_owned(), meta);
                    }
                }
                findings.extend(skill::audit_skill(&files));
            }
        }

        findings
            .into_iter()
            .map(|f| {
                let line = u64::try_from(f.line.saturating_sub(1)).unwrap_or(0);
                json!({
                    "range": {
                        "start": { "line": line, "character": 0 },
                        "end":   { "line": line, "character": u32::MAX },
                    },
                    // CRITICAL and HIGH are errors; the rest are warnings.
                    "severity": if f.severity == "CRITICAL" || f.severity == "HIGH" { 1 } else { 2 },
                    "code": f.rule,
                    "codeDescription": {
                        "href": format!(
                            "https://github.com/sebastienrousseau/agtmls-spec/blob/main/rules/{}.toml",
                            f.rule
                        ),
                    },
                    "source": "agtmls",
                    "message": f.message,
                })
            })
            .collect()
    }

    fn diagnostics_notification(&self, uri: &str) -> Value {
        let text = self
            .documents
            .get(uri)
            .map(String::as_str)
            .unwrap_or_default();
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": { "uri": uri, "diagnostics": self.diagnostics(uri, text) },
        })
    }

    /// Markdown headings, as document symbols.
    fn symbols(text: &str) -> Vec<Value> {
        text.lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let depth = line.chars().take_while(|c| *c == '#').count();
                (depth > 0 && line.chars().nth(depth) == Some(' ')).then(|| {
                    let line_no = u64::try_from(index).unwrap_or(0);
                    json!({
                        "name": line[depth + 1..].trim(),
                        "kind": 15,
                        "range": {
                            "start": {"line": line_no, "character": 0},
                            "end": {"line": line_no, "character": u32::MAX},
                        },
                        "selectionRange": {
                            "start": {"line": line_no, "character": 0},
                            "end": {"line": line_no, "character": u32::MAX},
                        },
                    })
                })
            })
            .collect()
    }

    /// The loaded rule set, if any.
    #[must_use]
    pub const fn rules(&self) -> Option<&RuleSet> {
        self.rules.as_ref()
    }
}
