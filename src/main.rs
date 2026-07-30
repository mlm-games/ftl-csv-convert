mod csv_io;
mod ftl;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use csv_io::{CsvTable, read_translations_csv, write_translations_csv};
use ftl::{is_valid_ftl_id, parse_ftl_file, render_ftl};

const DEFAULT_LOCALES_DIR: &str = "assets/locales";
const FTL_FILENAME: &str = "main.ftl";

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args: Vec<String> = env::args().skip(1).collect();

    let mut locales_dir = PathBuf::from(DEFAULT_LOCALES_DIR);
    let mut polyglot_headers = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" | "help" => {
                print_usage();
                return Ok(());
            }
            "-d" | "--locales-dir" => {
                let p = args
                    .get(i + 1)
                    .ok_or("--locales-dir requires a path")?
                    .clone();
                locales_dir = PathBuf::from(p);
                args.drain(i..=i + 1);
            }
            s if s.starts_with("--locales-dir=") => {
                locales_dir = PathBuf::from(&s["--locales-dir=".len()..]);
                args.remove(i);
            }
            "--polyglot" => {
                polyglot_headers = true;
                args.remove(i);
            }
            _ => i += 1,
        }
    }

    let (cmd, rest): (&str, &[String]) = match args.first().map(String::as_str) {
        None | Some("ftl2csv") => {
            let rest = if args.first().map(|s| s.as_str()) == Some("ftl2csv") {
                &args[1..]
            } else {
                &args[..]
            };
            ("ftl2csv", rest)
        }
        Some("csv2ftl") => ("csv2ftl", &args[1..]),
        Some(other) => {
            return Err(format!(
                "unknown command '{other}'. Use ftl2csv or csv2ftl (see --help)."
            ));
        }
    };

    match cmd {
        "ftl2csv" => {
            let out = rest.first().map(Path::new);
            cmd_ftl2csv(&locales_dir, out, polyglot_headers)
        }
        "csv2ftl" => {
            let csv_path = rest.first().ok_or("csv2ftl requires a CSV file path")?;
            cmd_csv2ftl(&locales_dir, Path::new(csv_path))
        }
        _ => unreachable!(),
    }
}

fn print_usage() {
    eprint!(
        "\
ftl-csv-convert — Fluent (.ftl) ↔ CSV

USAGE:
    ftl-csv-convert [OPTIONS] [ftl2csv] [OUTPUT.csv]
    ftl-csv-convert [OPTIONS] csv2ftl INPUT.csv

COMMANDS:
    ftl2csv     Export locales/*/main.ftl → CSV (default).
                Writes OUTPUT.csv or stdout.
    csv2ftl     Import CSV → locales/{{locale}}/main.ftl

OPTIONS:
    -d, --locales-dir <PATH>   Locales root (default: assets/locales)
        --polyglot             Export headers as STRING ID,DESCRIPTION,...
    -h, --help

CSV:
    Accepts either:
      key,context,en,es,...
      STRING ID,DESCRIPTION,en,fr,es,...   (Godot / just-keep-chasing polyglot)
    Empty cells = missing translation. UTF-8; BOM stripped on read; no BOM on write.
"
    );
}

fn cmd_ftl2csv(
    locales_dir: &Path,
    out_path: Option<&Path>,
    polyglot_headers: bool,
) -> Result<(), String> {
    let table = load_locales_dir(locales_dir)?;
    if table.locales.is_empty() {
        eprintln!(
            "warning: no locale dirs with {FTL_FILENAME} under {}",
            locales_dir.display()
        );
    }

    let mut dest: Box<dyn Write> = match out_path {
        Some(p) => {
            Box::new(fs::File::create(p).map_err(|e| format!("create {}: {e}", p.display()))?)
        }
        None => Box::new(io::stdout()),
    };

    write_translations_csv(&mut dest, &table, polyglot_headers)
        .map_err(|e| format!("write CSV: {e}"))?;

    if let Some(p) = out_path {
        eprintln!(
            "wrote {} keys × {} locales → {}",
            table.keys.len(),
            table.locales.len(),
            p.display()
        );
    }
    Ok(())
}

fn load_locales_dir(locales_dir: &Path) -> Result<CsvTable, String> {
    if !locales_dir.is_dir() {
        return Err(format!(
            "locales directory not found: {}",
            locales_dir.display()
        ));
    }

    let mut locales = Vec::new();
    let mut by_locale: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut contexts: BTreeMap<String, String> = BTreeMap::new();
    let mut all_keys: BTreeSet<String> = BTreeSet::new();

    let mut entries: Vec<_> = fs::read_dir(locales_dir)
        .map_err(|e| format!("read {}: {e}", locales_dir.display()))?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let locale = match entry.file_name().into_string() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("warning: skipping non-UTF-8 directory name");
                continue;
            }
        };

        let ftl_path = path.join(FTL_FILENAME);
        if !ftl_path.is_file() {
            eprintln!("warning: skipping locale '{locale}': missing {FTL_FILENAME}");
            continue;
        }

        let content = fs::read_to_string(&ftl_path)
            .map_err(|e| format!("read {}: {e}", ftl_path.display()))?;

        let parsed = parse_ftl_file(&content, &ftl_path);
        for (k, ctx) in parsed.contexts {
            contexts.entry(k).or_insert(ctx);
        }
        for k in parsed.messages.keys() {
            all_keys.insert(k.clone());
        }
        locales.push(locale.clone());
        by_locale.insert(locale, parsed.messages);
    }

    locales.sort();

    let mut translations: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for key in &all_keys {
        let mut row = BTreeMap::new();
        for loc in &locales {
            if let Some(v) = by_locale.get(loc).and_then(|m| m.get(key)) {
                row.insert(loc.clone(), v.clone());
            }
        }
        translations.insert(key.clone(), row);
    }

    Ok(CsvTable {
        locales,
        keys: all_keys.into_iter().collect(),
        translations,
        contexts,
    })
}

fn cmd_csv2ftl(locales_dir: &Path, csv_path: &Path) -> Result<(), String> {
    let raw = fs::read(csv_path).map_err(|e| format!("read {}: {e}", csv_path.display()))?;
    let table = read_translations_csv(&raw, csv_path)?;

    if table.locales.is_empty() {
        return Err("CSV has no locale columns".into());
    }

    fs::create_dir_all(locales_dir)
        .map_err(|e| format!("create {}: {e}", locales_dir.display()))?;

    for loc in &table.locales {
        let mut messages: BTreeMap<String, String> = BTreeMap::new();
        for key in &table.keys {
            if !is_valid_ftl_id(key) {
                continue;
            }
            if let Some(v) = table.translations.get(key).and_then(|m| m.get(loc))
                && !v.is_empty()
            {
                messages.insert(key.clone(), v.clone());
            }
        }

        let dir = locales_dir.join(loc);
        fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        let ftl_path = dir.join(FTL_FILENAME);
        let body = render_ftl(&messages, &table.contexts);
        fs::write(&ftl_path, body).map_err(|e| format!("write {}: {e}", ftl_path.display()))?;
        eprintln!("wrote {} messages → {}", messages.len(), ftl_path.display());
    }

    Ok(())
}
