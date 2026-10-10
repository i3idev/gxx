//! SQLite access: open, schema, load/save entry, search.

use rusqlite::Connection;
use serde_json::Value;
use std::path::PathBuf;

use crate::error::GxxError;
use crate::model::{entry_id, key_of, lang_of, normalise, now, searchable, title_of};

/// Schema SQL for the entries table.
pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS entries (
  id         TEXT PRIMARY KEY,
  kind       TEXT NOT NULL,
  lang       TEXT NOT NULL,
  level      TEXT,
  title      TEXT NOT NULL,
  text       TEXT,
  data       TEXT NOT NULL,
  created_at TEXT,
  updated_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_kind_lang ON entries(kind, lang);
";

/// Database name regex: letters, digits, - and _ only.
pub const DB_NAME_RE: &str = r"^[\w-]+$";

/// Home directory: GXX_HOME env or ~/.gxx
pub fn home_dir() -> PathBuf {
    std::env::var_os("GXX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut p = dirs::home_dir().expect("home directory");
            p.push(".gxx");
            p
        })
}

/// DB directory: ~/.gxx/db
pub fn db_dir() -> PathBuf {
    let mut p = home_dir();
    p.push("db");
    p
}

/// Config file: ~/.gxx/config (stores active db name)
pub fn config_path() -> PathBuf {
    let mut p = home_dir();
    p.push("config");
    p
}

pub fn active_name(override_name: Option<&str>) -> Result<String, GxxError> {
    if let Some(name) = override_name {
        return Ok(name.to_string());
    }
    let cfg = config_path();
    if !cfg.exists() {
        return Err(GxxError::msg(
            "no active db. run: gxx init, then: gxx db new <name>",
        ));
    }
    let name = std::fs::read_to_string(&cfg)?.trim().to_string();
    if name.is_empty() {
        Err(GxxError::msg(
            "no active db. run: gxx init, then: gxx db new <name>",
        ))
    } else {
        Ok(name)
    }
}

pub fn connect(override_name: Option<&str>) -> Result<Connection, GxxError> {
    let name = active_name(override_name)?;
    let path = db_dir().join(format!("{}.db", name));
    if !path.exists() {
        return Err(GxxError::msg(format!(
            "no database '{}'. see: gxx db list",
            name
        )));
    }
    let con = Connection::open(path)?;
    con.execute_batch(SCHEMA)?;
    Ok(con)
}

pub fn db_names() -> Result<Vec<String>, GxxError> {
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(db_dir()) {
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) == Some("db") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Open (or create) a database by name, return its path.
pub fn db_path(name: &str) -> PathBuf {
    db_dir().join(format!("{}.db", name))
}

/// Load a record by exact DB id (kind:key_of(name)).
/// Returns the JSON data as serde_json::Value.
pub fn load_rec(con: &Connection, kind: &str, name: &str) -> Result<Option<Value>, GxxError> {
    let id = entry_id(kind, name);
    let mut stmt = con.prepare("SELECT data FROM entries WHERE id = ?1")?;
    let mut rows = stmt.query([&id])?;
    if let Some(row) = rows.next()? {
        let json_str: String = row.get(0)?;
        Ok(Some(serde_json::from_str(&json_str)?))
    } else {
        Ok(None)
    }
}

/// Load a record with normalised-key fallback.
/// If exact-id lookup fails, scan all rows of that kind and match
/// normalised(key_of(title)) against normalised(key_of(name)).
/// This handles Persian/Arabic letter variants and ZWNJ.
pub fn load_rec_fallback(con: &Connection, kind: &str, name: &str) -> Result<Value, GxxError> {
    if let Some(rec) = load_rec(con, kind, name)? {
        return Ok(rec);
    }
    // Fallback: scan all rows of this kind, compare normalised keys.
    let norm_name = normalise(&key_of(name));
    let mut stmt = con.prepare("SELECT data FROM entries WHERE kind = ?1")?;
    let mut rows = stmt.query([kind])?;
    while let Some(row) = rows.next()? {
        let json_str: String = row.get(0)?;
        let rec: Value = serde_json::from_str(&json_str)?;
        let title = title_of(&rec);
        if normalise(&key_of(&title)) == norm_name {
            return Ok(rec);
        }
    }
    Err(GxxError::msg(format!(
        "no {} '{}'. add it first: gxx {} add {}",
        label(kind),
        name,
        label(kind),
        name
    )))
}

fn label(kind: &str) -> &str {
    crate::model::label(kind)
}

/// Save a record (upsert). `rec` is a JSON Value (the Python dict).
/// The text column is normalised; display text is never changed.
pub fn save_rec(con: &Connection, kind: &str, rec: &mut Value) -> Result<(), GxxError> {
    let n = now();
    rec["updated_at"] = Value::String(n.clone());
    if rec.get("created_at").is_none() {
        rec["created_at"] = Value::String(n.clone());
    }
    let title = title_of(rec);
    let id = entry_id(kind, &title);
    let lang = lang_of(&title);
    let level = rec
        .get("level")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let text = searchable(rec);
    let data_str = serde_json::to_string(rec)?;
    let created_at = rec["created_at"].as_str().unwrap_or("").to_string();
    let updated_at = rec["updated_at"].as_str().unwrap_or("").to_string();
    con.execute(
        "INSERT INTO entries(id, kind, lang, level, title, text, data, created_at, updated_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)
         ON CONFLICT(id) DO UPDATE SET
           lang=excluded.lang, level=excluded.level, title=excluded.title,
           text=excluded.text, data=excluded.data, updated_at=excluded.updated_at",
        (
            &id,
            kind,
            lang,
            level,
            &title,
            &text,
            &data_str,
            &created_at,
            &updated_at,
        ),
    )?;
    Ok(())
}

/// Build a LIKE pattern from a query string for SQL.
/// Python: if '*' in query -> query.replace('*','%') else '%query%'.
fn like_pattern(q: &str) -> String {
    if q.contains('*') {
        q.replace('*', "%")
    } else {
        format!("%{}%", q)
    }
}

/// Search entries in a single database.
/// Returns lines matching Python's `search()` output.
pub fn search(con: &Connection, q: &str) -> Result<Vec<String>, GxxError> {
    let ql = q.to_lowercase();
    let _pat = like_pattern(&ql);

    // We could use SQL LIKE prefilter, but to correctly handle normalisation
    // on old rows we do the full scan in Rust and filter on normalised text.
    // The table is small.
    let mut stmt = con.prepare("SELECT kind, data FROM entries")?;
    let mut rows = stmt.query([])?;

    let mut matches = Vec::new();
    let nq = normalise(&ql);
    while let Some(row) = rows.next()? {
        let kind: String = row.get(0)?;
        let json_str: String = row.get(1)?;
        let rec: Value = serde_json::from_str(&json_str)?;

        let title = title_of(&rec);
        let st = searchable(&rec);
        // Match if normalised query is substring of normalised title OR searchable
        // For '*' wildcard, use a glob-like match on normalised text.
        let title_match = if ql.contains('*') {
            glob_match(&nq, &normalise(&title))
        } else {
            normalise(&title).contains(&nq)
        };
        let text_match = if ql.contains('*') {
            glob_match(&nq, &st)
        } else {
            st.contains(&nq)
        };

        if title_match || text_match {
            let line = match kind.as_str() {
                "word" => {
                    let trs: Vec<String> = rec
                        .get("translations")
                        .and_then(|v| v.as_object())
                        .map(|m| {
                            m.values()
                                .flat_map(|v| v.as_array())
                                .flatten()
                                .filter_map(|x| x.as_str())
                                .take(4)
                                .map(|s| s.to_string())
                                .collect()
                        })
                        .unwrap_or_default();
                    let pos = rec
                        .get("pos")
                        .and_then(|v| v.as_str())
                        .map(|s| format!(" ({})", s))
                        .unwrap_or_default();
                    format!("[word] {}{pos} : {}", title, trs.join(", "))
                }
                _ => {
                    let rule = rec.get("rule").and_then(|v| v.as_str()).unwrap_or("");
                    format!("[{}] {} : {}", label(&kind), title, rule)
                }
            };
            matches.push((title, line));
        }
    }

    // Sort: exact match first (key_of(title) != ql), then title in code-point order.
    matches.sort_by(|a, b| {
        let a_exact = key_of(&a.0) != ql;
        let b_exact = key_of(&b.0) != ql;
        a_exact.cmp(&b_exact).then_with(|| a.0.cmp(&b.0))
    });

    Ok(matches.into_iter().map(|m| m.1).collect())
}

/// Search entries and return complete records as structured JSON values.
/// Uses the same matching and ordering rules as `search()`.
pub fn search_records(con: &Connection, q: &str) -> Result<Vec<Value>, GxxError> {
    let ql = q.to_lowercase();
    let nq = normalise(&ql);

    let mut stmt = con.prepare("SELECT kind, data FROM entries")?;
    let mut rows = stmt.query([])?;
    let mut matches: Vec<(String, Value)> = Vec::new();

    while let Some(row) = rows.next()? {
        let json_str: String = row.get(1)?;
        let rec: Value = serde_json::from_str(&json_str)?;

        let title = title_of(&rec);
        let st = searchable(&rec);

        let title_match = if ql.contains('*') {
            glob_match(&nq, &normalise(&title))
        } else {
            normalise(&title).contains(&nq)
        };
        let text_match = if ql.contains('*') {
            glob_match(&nq, &st)
        } else {
            st.contains(&nq)
        };

        if title_match || text_match {
            matches.push((title, rec));
        }
    }

    matches.sort_by(|a, b| {
        let a_exact = key_of(&a.0) != ql;
        let b_exact = key_of(&b.0) != ql;
        a_exact.cmp(&b_exact).then_with(|| a.0.cmp(&b.0))
    });

    Ok(matches.into_iter().map(|(_, rec)| rec).collect())
}

/// Simple glob match: '*' matches any sequence, other chars literal.
/// Applied to already-normalised strings.
fn glob_match(pattern: &str, text: &str) -> bool {
    // Convert to regex: escape, then * -> .*
    let mut regex_pat = String::with_capacity(pattern.len() * 2);
    for ch in pattern.chars() {
        match ch {
            '*' => regex_pat.push_str(".*"),
            c if ".^$+?()[]{}|\\".contains(c) => {
                regex_pat.push('\\');
                regex_pat.push(c);
            }
            c => regex_pat.push(c),
        }
    }
    // Anchored
    let regex_pat = format!("^{}$", regex_pat);
    regex::Regex::new(&regex_pat).unwrap().is_match(text)
}
