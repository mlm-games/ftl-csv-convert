use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;

use crate::ftl::is_valid_ftl_id;

const KEY_HEADERS: &[&str] = &["key", "string id", "string_id", "id", "msgctxt"];
const CONTEXT_HEADERS: &[&str] = &[
    "context",
    "description",
    "comment",
    "comments",
    "desc",
    "notes",
];

pub struct CsvTable {
    pub locales: Vec<String>,
    pub keys: Vec<String>,
    pub translations: BTreeMap<String, BTreeMap<String, String>>,
    pub contexts: BTreeMap<String, String>,
}

pub fn write_translations_csv<W: Write>(
    w: &mut W,
    table: &CsvTable,
    polyglot_headers: bool,
) -> Result<(), csv::Error> {
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(b',')
        .quote_style(csv::QuoteStyle::Necessary)
        .from_writer(w);

    let has_context = !table.contexts.is_empty();

    let mut header: Vec<String> = Vec::new();
    if polyglot_headers {
        header.push("STRING ID".into());
        if has_context {
            header.push("DESCRIPTION".into());
        }
    } else {
        header.push("key".into());
        if has_context {
            header.push("context".into());
        }
    }
    header.extend(table.locales.iter().cloned());
    wtr.write_record(&header)?;

    for key in &table.keys {
        let mut row: Vec<String> = vec![key.clone()];
        if has_context {
            row.push(table.contexts.get(key).cloned().unwrap_or_default());
        }
        for loc in &table.locales {
            let val = table
                .translations
                .get(key)
                .and_then(|m| m.get(loc))
                .cloned()
                .unwrap_or_default();
            row.push(val);
        }
        wtr.write_record(&row)?;
    }
    wtr.flush()?;
    Ok(())
}

pub fn read_translations_csv(raw: &[u8], path: &Path) -> Result<CsvTable, String> {
    let data = strip_bom(raw);
    if data.1 {
        eprintln!("note: stripped UTF-8 BOM from {}", path.display());
    }

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .flexible(true)
        .from_reader(data.0);

    let headers = rdr
        .headers()
        .map_err(|e| format!("CSV headers: {e}"))?
        .clone();

    if headers.is_empty() {
        return Err("CSV has no header row".into());
    }

    let headers_lower: Vec<String> = headers
        .iter()
        .map(|h| h.trim().to_ascii_lowercase())
        .collect();

    let key_idx = headers_lower
        .iter()
        .position(|h| KEY_HEADERS.contains(&h.as_str()))
        .unwrap_or(0);

    if !KEY_HEADERS.contains(&headers_lower[key_idx].as_str()) {
        eprintln!(
            "warning: first/key column is '{}'; expected 'key' or 'STRING ID'",
            headers.get(key_idx).unwrap_or("")
        );
    }

    let context_idx = headers_lower.iter().enumerate().find_map(|(i, h)| {
        if i != key_idx && CONTEXT_HEADERS.contains(&h.as_str()) {
            Some(i)
        } else {
            None
        }
    });

    let mut locale_cols: Vec<(usize, String)> = Vec::new();
    for (i, h) in headers.iter().enumerate() {
        if i == key_idx || Some(i) == context_idx {
            continue;
        }
        let name = h.trim();
        if name.is_empty() {
            eprintln!("warning: empty locale header at column {i}, skipping");
            continue;
        }
        let lower = name.to_ascii_lowercase();
        if CONTEXT_HEADERS.contains(&lower.as_str()) || KEY_HEADERS.contains(&lower.as_str()) {
            continue;
        }
        locale_cols.push((i, name.to_string()));
    }

    let locales: Vec<String> = locale_cols.iter().map(|(_, n)| n.clone()).collect();

    let mut translations: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut contexts: BTreeMap<String, String> = BTreeMap::new();
    let mut keys = BTreeSet::new();

    for (row_num, result) in rdr.records().enumerate() {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: skipping malformed CSV row {}: {e}", row_num + 2);
                continue;
            }
        };

        let key = record.get(key_idx).unwrap_or("").trim();
        if key.is_empty() {
            continue;
        }

        if !is_valid_ftl_id(key) {
            eprintln!(
                "warning: key '{key}' on row {} is not a valid FTL identifier; \
                 it will be skipped on FTL write",
                row_num + 2
            );
        }

        keys.insert(key.to_string());

        if let Some(ci) = context_idx {
            if let Some(c) = record.get(ci) {
                let c = c.trim();
                if !c.is_empty() {
                    contexts.insert(key.to_string(), c.to_string());
                }
            }
        }

        let mut row_map = BTreeMap::new();
        for (col, loc) in &locale_cols {
            let val = record.get(*col).unwrap_or("");
            if val.is_empty() {
                continue;
            }
            row_map.insert(loc.clone(), val.to_string());
        }
        translations.insert(key.to_string(), row_map);
    }

    Ok(CsvTable {
        locales,
        keys: keys.into_iter().collect(),
        translations,
        contexts,
    })
}

fn strip_bom(raw: &[u8]) -> (&[u8], bool) {
    if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (&raw[3..], true)
    } else {
        (raw, false)
    }
}
