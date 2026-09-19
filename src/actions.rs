// SPDX-FileCopyrightText: 2026 Sebastien Rousseau
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Code actions.
//!
//! Every action here resolves a finding in the **safe** direction. Where an
//! unsafe resolution exists — widening a safety policy to match a frontmatter
//! grant, rather than narrowing the grant — it is deliberately not offered:
//! an editor that makes granting Bash a one-keystroke fix has not helped.

use agtmls_core::skill::frontmatter_tools;
use serde_json::{Value, json};

/// Tools whose capability a `safety_policy` may deny.
const ESCALATING: &[&str] = &[
    "Bash",
    "BashOutput",
    "KillShell",
    "Write",
    "Edit",
    "NotebookEdit",
    "WebFetch",
    "WebSearch",
];

fn edit(uri: &str, line: usize, text: &str) -> Value {
    json!({
        "changes": {
            uri: [{
                "range": {
                    "start": { "line": line, "character": 0 },
                    "end":   { "line": line, "character": u32::MAX },
                },
                "newText": text,
            }],
        },
    })
}

/// Actions for the diagnostics the client reported at the cursor.
#[must_use]
pub fn for_diagnostics(uri: &str, text: &str, diagnostics: &Value) -> Vec<Value> {
    let lines: Vec<&str> = text.lines().collect();
    let mut actions = Vec::new();

    for diagnostic in diagnostics.as_array().into_iter().flatten() {
        let rule = diagnostic["code"].as_str().unwrap_or_default();
        let line = usize::try_from(diagnostic["range"]["start"]["line"].as_u64().unwrap_or(0))
            .unwrap_or(0);

        match rule {
            "AGT-STEG-001" => {
                let Some(source) = lines.get(line) else {
                    continue;
                };
                let cleaned: String = source.chars().filter(|c| !is_invisible(*c)).collect();
                if cleaned != *source {
                    let removed = source.chars().count() - cleaned.chars().count();
                    actions.push(json!({
                        "title": format!("Remove {removed} invisible character(s)"),
                        "kind": "quickfix",
                        "diagnostics": [diagnostic],
                        "isPreferred": true,
                        "edit": edit(uri, line, &cleaned),
                    }));
                }
            }
            "AGT-CAP-001" => {
                // Narrow the grant, never widen the policy. The opposite fix
                // resolves the same diagnostic by handing the skill the
                // capability it was caught claiming not to need.
                let Some((index, source)) = lines
                    .iter()
                    .enumerate()
                    .find(|(_, l)| l.trim_start().starts_with("allowed-tools"))
                else {
                    continue;
                };
                let kept: Vec<String> = frontmatter_tools(text)
                    .into_iter()
                    .filter(|t| !ESCALATING.contains(&t.as_str()))
                    .collect();
                let indent = &source[..source.len() - source.trim_start().len()];
                actions.push(json!({
                    "title": "Narrow allowed-tools to what safety_policy permits",
                    "kind": "quickfix",
                    "diagnostics": [diagnostic],
                    "isPreferred": true,
                    "edit": edit(uri, index, &format!("{indent}allowed-tools: {}", kept.join(", "))),
                }));
            }
            "AGT-POLICY-001" => {
                actions.push(json!({
                    "title": "Create metadata.json from the template (review-gated)",
                    "kind": "quickfix",
                    "diagnostics": [diagnostic],
                    "command": {
                        "title": "Create metadata.json",
                        "command": "agtmls.createMetadata",
                        "arguments": [uri],
                    },
                }));
            }
            _ => {}
        }
    }
    actions
}

fn is_invisible(c: char) -> bool {
    matches!(c as u32,
        0x00AD | 0x034F | 0x061C | 0x115F | 0x1160 | 0x17B4 | 0x17B5 | 0x180E
        | 0x200B..=0x200F | 0x2060..=0x2064 | 0x2066..=0x2069
        | 0x202A..=0x202E | 0x3164 | 0xFEFF | 0xFFA0
        | 0xFE00..=0xFE0F | 0xE0000..=0xE007F)
}
