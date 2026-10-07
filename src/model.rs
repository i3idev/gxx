//! Data model: Entry/Word/Grammar structs and helpers.
//!
//! The JSON shape must be semantically compatible with the Python version:
//! known fields are typed, unknown fields are preserved via `serde_json::Value`
//! so Python-made databases round-trip through Rust and vice versa.

use chrono::Utc;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::Mutex;

/// Global quiet state for load command.
pub static STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State { quiet: false }));

pub struct State {
    pub quiet: bool,
}

pub fn set_quiet(q: bool) {
    STATE.lock().unwrap().quiet = q;
}

/// Persian normalisation applied only to search keys / the `text` column.
///
/// Arabic ي -> ی, ك -> ک, and ZWNJ (U+200C) is removed so that lookups that
/// differ only by these characters still match.  Display text is never
/// changed.
pub fn normalise(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\u{064A}' => out.push('\u{06CC}'), // Arabic yeh -> Persian yeh
            '\u{0643}' => out.push('\u{06A9}'), // Arabic kaf -> Persian kaf
            '\u{200C}' => { /* ZWNJ: drop */ }
            other => out.push(other),
        }
    }
    out
}

/// Persian if any char is in U+0600..U+06FF, else English.
pub fn lang_of(text: &str) -> &'static str {
    if text.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)) {
        "fa"
    } else {
        "en"
    }
}

/// Underscores stand for spaces; then lowercased.  Used for entry IDs —
/// IDs must NOT be normalised so Python-made databases stay byte-compatible.
pub fn key_of(name: &str) -> String {
    name.replace('_', " ").trim().to_lowercase()
}

/// Lowercased, whitespace collapsed to `-`.
pub fn slug(name: &str) -> String {
    key_of(name)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// `word` -> "word", `grammar` -> "gram", `style` -> "style", `form` -> "form".
pub fn label(kind: &str) -> &str {
    match kind {
        "word" => "word",
        "grammar" => "gram",
        "style" => "style",
        "form" => "form",
        _ => "unknown",
    }
}

/// `rec["word"]` or `rec["title"]` or "".
pub fn title_of(rec: &Value) -> String {
    rec.get("word")
        .and_then(|v| v.as_str())
        .or_else(|| rec.get("title").and_then(|v| v.as_str()))
        .unwrap_or_default()
        .to_string()
}

/// Underscores to spaces, trim.
pub fn clean(token: &str) -> String {
    token.replace('_', " ").trim().to_string()
}

/// Current UTC time in ISO format without microseconds.
pub fn now() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Add item to array if not already present (case-insensitive).
pub fn add_unique(arr: &mut Vec<Value>, item: String) {
    if item.is_empty() {
        return;
    }
    let lower = item.to_lowercase();
    if !arr.iter().any(|x| {
        x.as_str()
            .map(|s| s.to_lowercase() == lower)
            .unwrap_or(false)
    }) {
        arr.push(Value::String(item));
    }
}

/// Remove item from array (case-insensitive). Returns true if removed.
pub fn remove_item(arr: &mut Vec<Value>, item: &str) -> bool {
    if item.is_empty() {
        return false;
    }
    let lower = item.to_lowercase();
    for i in (0..arr.len()).rev() {
        if arr[i]
            .as_str()
            .map(|s| s.to_lowercase() == lower)
            .unwrap_or(false)
        {
            arr.remove(i);
            return true;
        }
    }
    false
}

/// Build the searchable `text` column from a record, normalised.
///
/// Computed in Rust from the row's JSON (not from the stored column) so that
/// old rows written by Python without normalisation still match.
pub fn searchable(rec: &Value) -> String {
    let mut parts: Vec<String> = Vec::new();
    let t = title_of(rec);
    if !t.is_empty() {
        parts.push(t);
    }
    for k in ["def", "rule", "desc"] {
        if let Some(s) = rec.get(k).and_then(|v| v.as_str()) {
            parts.push(s.to_string());
        }
    }
    if let Some(map) = rec.get("translations").and_then(|v| v.as_object()) {
        for values in map.values() {
            if let Some(arr) = values.as_array() {
                for x in arr {
                    if let Some(s) = x.as_str() {
                        parts.push(s.to_string());
                    }
                }
            }
        }
    }
    for k in ["synonyms", "antonyms"] {
        if let Some(arr) = rec.get(k).and_then(|v| v.as_array()) {
            for x in arr {
                if let Some(s) = x.as_str() {
                    parts.push(s.to_string());
                }
            }
        }
    }
    normalise(&parts.join(" "))
}

/// Create a new word record (like Python's new_word).
pub fn new_word(name: &str) -> Value {
    serde_json::json!({
        "id": format!("{}-{}", lang_of(name), slug(name)),
        "word": name,
        "translations": {},
        "synonyms": [],
        "antonyms": [],
        "examples": [],
        "notes": ""
    })
}

/// Create a new grammar/style/form record.
pub fn new_grammar(_kind: &str, name: &str) -> Value {
    serde_json::json!({
        "id": format!("{}-{}", lang_of(name), slug(name)),
        "title": name,
        "examples": [],
        "errors": [],
        "notes": ""
    })
}

/// Entry ID: "<kind>:<key_of(name)>"
pub fn entry_id(kind: &str, name: &str) -> String {
    format!("{}:{}", kind, key_of(name))
}

/// Ensure senses array exists and return mutable reference.
pub fn senses_mut(rec: &mut Value) -> &mut Vec<Value> {
    if rec.get("senses").is_none() {
        rec["senses"] = Value::Array(vec![]);
    }
    rec["senses"].as_array_mut().unwrap()
}

/// Ensure collocations array exists and return mutable reference.
pub fn collocations_mut(rec: &mut Value) -> &mut Vec<Value> {
    if rec.get("collocations").is_none() {
        rec["collocations"] = Value::Array(vec![]);
    }
    rec["collocations"].as_array_mut().unwrap()
}

/// Ensure family array exists and return mutable reference.
pub fn family_mut(rec: &mut Value) -> &mut Vec<Value> {
    if rec.get("family").is_none() {
        rec["family"] = Value::Array(vec![]);
    }
    rec["family"].as_array_mut().unwrap()
}

/// Create a new sense object.
pub fn new_sense(def: &str) -> Value {
    serde_json::json!({
        "def": def,
        "note": "",
        "examples": []
    })
}

/// Create a new family member object.
pub fn new_family_member(word: &str, pos: Option<&str>) -> Value {
    let mut obj = serde_json::json!({"word": word});
    if let Some(p) = pos {
        if !p.is_empty() {
            obj["pos"] = Value::String(p.to_string());
        }
    }
    obj
}
