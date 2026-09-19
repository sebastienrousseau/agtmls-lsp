// SPDX-FileCopyrightText: 2026 Sebastien Rousseau
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Frontmatter completion.
//!
//! The keys and the `allowed-tools` values are the two things an author
//! currently has to remember or look up, and getting `allowed-tools` wrong is
//! not a typo — it is a capability grant.

use serde_json::{Value, json};

/// Frontmatter keys, with what each one is for.
const KEYS: &[(&str, &str)] = &[
    ("name", "Skill name. Must match the directory name."),
    (
        "description",
        "What the skill is for, and when to load it. This is the text a router matches against, so write the trigger, not the summary.",
    ),
    (
        "allowed-tools",
        "Tools the runtime grants this skill. The runtime honours this, not metadata.json, so it must not exceed what safety_policy admits (AGT-CAP-001).",
    ),
    ("license", "SPDX expression."),
    (
        "compatibility",
        "Runtimes this skill has been tested against.",
    ),
];

/// Tools, and the capability each implies.
const TOOLS: &[(&str, &str)] = &[
    ("Read", "Read files. No capability implied."),
    ("Glob", "Match paths. No capability implied."),
    ("Grep", "Search contents. No capability implied."),
    (
        "Write",
        "Create files — requires safety_policy.writes_files.",
    ),
    (
        "Edit",
        "Modify files — requires safety_policy.writes_files.",
    ),
    (
        "Bash",
        "Run shell commands — requires safety_policy.executes_commands.",
    ),
    (
        "WebFetch",
        "Fetch a URL — requires safety_policy.network_access.",
    ),
    (
        "WebSearch",
        "Search the web — requires safety_policy.network_access.",
    ),
];

fn item(label: &str, detail: &str, kind: u8) -> Value {
    json!({ "label": label, "kind": kind, "detail": detail })
}

/// Completions for the cursor's line.
///
/// Deliberately context-sensitive on one axis only — whether the line is an
/// `allowed-tools` line — because offering tool names where a key belongs is
/// worse than offering nothing.
#[must_use]
pub fn items(text: &str, line: usize) -> Vec<Value> {
    let lines: Vec<&str> = text.lines().collect();
    let current = lines.get(line).copied().unwrap_or("");

    // Only inside frontmatter: the body is prose and completing YAML keys
    // into it would be noise.
    if !in_frontmatter(&lines, line) {
        return Vec::new();
    }

    if current.trim_start().starts_with("allowed-tools") {
        return TOOLS
            .iter()
            .map(|(name, detail)| item(name, detail, 6))
            .collect();
    }
    if current.contains(':') {
        return Vec::new();
    }
    KEYS.iter()
        .filter(|(key, _)| !declared(&lines, key) || current.trim_start().starts_with(key))
        .map(|(key, detail)| item(key, detail, 14))
        .collect()
}

fn in_frontmatter(lines: &[&str], line: usize) -> bool {
    if lines.first().map(|l| l.trim()) != Some("---") {
        return false;
    }
    lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, l)| l.trim() == "---")
        .is_some_and(|(close, _)| line > 0 && line < close)
}

fn declared(lines: &[&str], key: &str) -> bool {
    lines
        .iter()
        .skip(1)
        .take_while(|l| l.trim() != "---")
        .any(|l| l.trim_start().starts_with(&format!("{key}:")))
}
