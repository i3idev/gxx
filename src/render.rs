//! All text output (show, lists, examples). Pure functions returning String.
//! Output is plain ASCII layout only — no colours, no emoji, no tables crate.

use serde_json::Value;
use std::path::Path;

use crate::model::{label, lang_of, title_of};

/// Format example lines exactly like Python.
/// Output:
///   EXAMPLES (N)
///    1> en text
///     = fa text
///
///    2> en text
///     = fa text
/// ...
/// (no trailing blank line)
pub fn ex_lines(examples: &[Value]) -> String {
    if examples.is_empty() {
        return "no examples".to_string();
    }
    let mut out = vec![format!("EXAMPLES ({})", examples.len())];
    for (i, ex) in examples.iter().enumerate() {
        let en = ex.get("en").and_then(|v| v.as_str()).unwrap_or("[no en]");
        let fa = ex.get("fa").and_then(|v| v.as_str()).unwrap_or("[no fa]");
        out.push(format!(" {}> {}", i + 1, en));
        out.push(format!("    = {}", fa));
        out.push(String::new());
    }
    // Remove trailing blank line
    if out.last() == Some(&String::new()) {
        out.pop();
    }
    out.join("\n")
}

/// Build blocks for each section (def, tr, syn, ant, ex, note for words;
/// rule, desc, ex, err, note for grammar/style/form).
/// Returns a map of section key -> lines.
pub fn entry_blocks(kind: &str, rec: &Value) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut b = std::collections::BTreeMap::new();
    if kind == "word" {
        if let Some(def) = rec.get("def").and_then(|v| v.as_str()) {
            b.insert("def".into(), vec![format!("DEF : {}", def)]);
        }
        if let Some(map) = rec.get("translations").and_then(|v| v.as_object()) {
            if !map.is_empty() {
                let mut tr_lines = vec!["TRANSLATIONS".to_string()];
                for (lang, vals) in map {
                    if let Some(arr) = vals.as_array() {
                        let joined: Vec<String> = arr
                            .iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect();
                        if !joined.is_empty() {
                            tr_lines.push(format!(" {} : {}", lang, joined.join(", ")));
                        }
                    }
                }
                if tr_lines.len() > 1 {
                    b.insert("tr".into(), tr_lines);
                }
            }
        }
        if let Some(arr) = rec.get("synonyms").and_then(|v| v.as_array()) {
            let syns: Vec<String> = arr
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect();
            if !syns.is_empty() {
                b.insert(
                    "syn".into(),
                    vec![format!("SYNONYMS : {}", syns.join(", "))],
                );
            }
        }
        if let Some(arr) = rec.get("antonyms").and_then(|v| v.as_array()) {
            let ants: Vec<String> = arr
                .iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect();
            if !ants.is_empty() {
                b.insert(
                    "ant".into(),
                    vec![format!("ANTONYMS : {}", ants.join(", "))],
                );
            }
        }
        // Senses
        if let Some(arr) = rec.get("senses").and_then(|v| v.as_array()) {
            if !arr.is_empty() {
                let mut sense_lines = vec!["SENSES".to_string()];
                for (i, s) in arr.iter().enumerate() {
                    if let Some(def) = s.get("def").and_then(|v| v.as_str()) {
                        sense_lines.push(format!(" {}> {}", i + 1, def));
                    }
                    if let Some(note) = s.get("note").and_then(|v| v.as_str()) {
                        if !note.is_empty() {
                            sense_lines.push(format!("    note: {}", note));
                        }
                    }
                    if let Some(ex_arr) = s.get("examples").and_then(|v| v.as_array()) {
                        if !ex_arr.is_empty() {
                            for ex in ex_arr {
                                let en = ex.get("en").and_then(|v| v.as_str()).unwrap_or("");
                                let fa = ex.get("fa").and_then(|v| v.as_str()).unwrap_or("");
                                if !en.is_empty() {
                                    sense_lines.push(format!("    ex: {}", en));
                                }
                                if !fa.is_empty() {
                                    sense_lines.push(format!("    ex: {}", fa));
                                }
                            }
                        }
                    }
                }
                b.insert("sense".into(), sense_lines);
            }
        }
        // Collocations
        if let Some(arr) = rec.get("collocations").and_then(|v| v.as_array()) {
            if !arr.is_empty() {
                let mut coll_lines = vec!["COLLOCATIONS".to_string()];
                for (i, c) in arr.iter().enumerate() {
                    if let Some(text) = c.as_str() {
                        coll_lines.push(format!(" {}> {}", i + 1, text));
                    }
                }
                b.insert("coll".into(), coll_lines);
            }
        }
        // Word Family
        if let Some(arr) = rec.get("family").and_then(|v| v.as_array()) {
            if !arr.is_empty() {
                let mut fam_lines = vec!["WORD FAMILY".to_string()];
                for m in arr {
                    if let Some(word) = m.get("word").and_then(|v| v.as_str()) {
                        let mut line = format!(" {}> {}", word, "");
                        if let Some(pos) = m.get("pos").and_then(|v| v.as_str()) {
                            if !pos.is_empty() {
                                line.push_str(&format!(" ({})", pos));
                            }
                        }
                        fam_lines.push(line);
                    }
                }
                b.insert("family".into(), fam_lines);
            }
        }
    } else {
        if let Some(rule) = rec.get("rule").and_then(|v| v.as_str()) {
            b.insert("rule".into(), vec![format!("RULE : {}", rule)]);
        }
        if let Some(desc) = rec.get("desc").and_then(|v| v.as_str()) {
            b.insert("desc".into(), vec![format!("DESC : {}", desc)]);
        }
        if let Some(arr) = rec.get("errors").and_then(|v| v.as_array()) {
            if !arr.is_empty() {
                let mut err_lines = vec![format!("ERRORS ({})", arr.len())];
                for (i, e) in arr.iter().enumerate() {
                    if let Some(t) = e.as_str() {
                        err_lines.push(format!(" {}> {}", i + 1, t));
                    }
                }
                b.insert("err".into(), err_lines);
            }
        }
    }
    if let Some(arr) = rec.get("examples").and_then(|v| v.as_array()) {
        if !arr.is_empty() {
            b.insert(
                "ex".into(),
                ex_lines(arr).lines().map(|s| s.to_string()).collect(),
            );
        }
    }
    let notes = rec.get("notes").and_then(|v| v.as_str()).unwrap_or("-");
    b.insert("note".into(), vec![format!("NOTES : {}", notes)]);
    b
}

/// Build the header line.
/// Word: "word  (pos)  [level]  /pronunciation/"
/// Grammar/Style/Form: "title  [gram|style|form]  [lang]  [level]  (cat)"
pub fn header(kind: &str, rec: &Value) -> String {
    let t = title_of(rec);
    if kind == "word" {
        let mut h = t.clone();
        if let Some(pos) = rec.get("pos").and_then(|v| v.as_str()) {
            h.push_str(&format!("  ({})", pos));
        }
        if let Some(lvl) = rec.get("level").and_then(|v| v.as_str()) {
            h.push_str(&format!("  [{}]", lvl));
        }
        if let Some(pron) = rec.get("pronunciation").and_then(|v| v.as_str()) {
            h.push_str(&format!("  /{}/", pron));
        }
        h
    } else {
        let mut h = format!("{}  [{}]  [{}]", t, label(kind), lang_of(&t));
        if let Some(lvl) = rec.get("level").and_then(|v| v.as_str()) {
            h.push_str(&format!("  [{}]", lvl));
        }
        if let Some(cat) = rec.get("cat").and_then(|v| v.as_str()) {
            h.push_str(&format!("  ({})", cat));
        }
        h
    }
}

/// Main render function.
/// If sections is empty, render full entry with bars and timestamps.
/// If sections non-empty, render only those sections (after title line).
/// Sections for word: def, tr, syn, ant, ex, sense, coll, family, note
/// Sections for grammar: rule, desc, ex, err, note
pub fn render_entry(kind: &str, rec: &Value, sections: &[String]) -> String {
    let blocks = entry_blocks(kind, rec);
    let (order, valid): (Vec<Vec<&str>>, Vec<&str>) = if kind == "word" {
        (
            vec![
                vec!["def"],
                vec!["tr"],
                vec!["syn", "ant"],
                vec!["ex"],
                vec!["sense"],
                vec!["coll"],
                vec!["family"],
                vec!["note"],
            ],
            vec![
                "def", "tr", "syn", "ant", "ex", "sense", "coll", "family", "note",
            ],
        )
    } else {
        (
            vec![vec!["rule", "desc"], vec!["ex"], vec!["err"], vec!["note"]],
            vec!["rule", "desc", "ex", "err", "note"],
        )
    };

    // Validate sections
    for s in sections {
        if !valid.iter().any(|&v| v == s) {
            return format!("error: unknown section '{}' (use: {})", s, valid.join(" "));
        }
    }

    let mut lines = Vec::new();

    if !sections.is_empty() {
        // Sectioned output: title only, then requested blocks
        lines.push(title_of(rec));
        for group in &order {
            for k in group {
                if sections.iter().any(|s| s == k) {
                    if let Some(block) = blocks.get(*k) {
                        lines.extend(block.clone());
                    }
                }
            }
        }
        return lines.join("\n");
    }

    // Full output
    let bar = "=".repeat(48);
    lines.push(bar.clone());
    lines.push(header(kind, rec));
    lines.push(bar.clone());

    for group in &order {
        let present: Vec<&str> = group
            .iter()
            .filter(|k| valid.iter().any(|v| v == *k))
            .copied()
            .collect();
        if !present.is_empty() {
            lines.push(String::new());
            for k in present {
                if let Some(block) = blocks.get(k) {
                    lines.extend(block.clone());
                }
            }
        }
    }

    let created = rec.get("created_at").and_then(|v| v.as_str()).unwrap_or("") as &str;
    let updated = rec.get("updated_at").and_then(|v| v.as_str()).unwrap_or("") as &str;
    let created = &created[..created.len().min(10)];
    let updated = &updated[..updated.len().min(10)];
    lines.push(String::new());
    lines.push(format!("CREATED: {}  UPDATED: {}", created, updated));
    lines.push(bar);

    lines.join("\n")
}

/// Render list output for words (cmd_list).
/// Format: "{title:<18} {pos or -:<10} {level or -:<4} {translations[:3]}"
pub fn render_word_list(rec: &Value) -> String {
    let title = title_of(rec);
    let pos = rec.get("pos").and_then(|v| v.as_str()).unwrap_or("-");
    let lvl = rec.get("level").and_then(|v| v.as_str()).unwrap_or("-");
    let trs: Vec<String> = rec
        .get("translations")
        .and_then(|v| v.as_object())
        .map(|m| {
            m.values()
                .flat_map(|v| v.as_array())
                .flatten()
                .filter_map(|x| x.as_str())
                .take(3)
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    format!("{:<18} {:<10} {:<4} {}", title, pos, lvl, trs.join(", "))
}

/// Render list output for grammar/style/form (cmd_glist).
/// Format: "{title:<28} {level or -:<4} {cat or -:<10} {rule}"
pub fn render_gram_list(rec: &Value) -> String {
    let title = title_of(rec);
    let lvl = rec.get("level").and_then(|v| v.as_str()).unwrap_or("-");
    let cat = rec.get("cat").and_then(|v| v.as_str()).unwrap_or("-");
    let rule = rec.get("rule").and_then(|v| v.as_str()).unwrap_or("");
    format!("{:<28} {:<4} {:<10} {}", title, lvl, cat, rule)
}

/// Render db list output.
/// Format: "{* or space} {name:<15} {count:>7}  {updated[:10]}"
pub fn render_db_list_line(
    active: &str,
    name: &str,
    count: i64,
    updated: Option<String>,
) -> String {
    let star = if name == active { "*" } else { " " };
    let upd = updated.unwrap_or_default();
    let upd = &upd[..upd.len().min(10)];
    format!("{} {:<15} {:>7}  {}", star, name, count, upd)
}

/// Render db info output.
pub fn render_db_info(
    name: &str,
    path: &Path,
    is_active: bool,
    counts: &[(&str, i64)],
) -> Vec<String> {
    let mut out = Vec::new();
    let active_suffix = if is_active { "  (active)" } else { "" };
    out.push(format!("name : {}{}", name, active_suffix));
    out.push(format!("path : {}", path.display()));
    for (kind, count) in counts {
        out.push(format!("{:<6}: {}", label(kind), count));
    }
    out
}
