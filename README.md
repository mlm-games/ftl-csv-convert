# ftl-csv-convert

Bidirectional converter: Fluent `assets/locales/{locale}/main.ftl` ↔ one CSV for spreadsheet editing.

Compatible with **Godot polyglot** CSVs (`STRING ID`, `DESCRIPTION`, …) such as
[just-keep-chasing/locale/translations.csv](https://github.com/mlm-games/just-keep-chasing/blob/main/locale/translations.csv).

## Build / run

```bash
# From repo root
cargo run --manifest-path tools/ftl-csv-convert/Cargo.toml -- ftl2csv
cargo run --manifest-path tools/ftl-csv-convert/Cargo.toml -- ftl2csv out.csv
cargo run --manifest-path tools/ftl-csv-convert/Cargo.toml -- --polyglot ftl2csv out.csv
cargo run --manifest-path tools/ftl-csv-convert/Cargo.toml -- csv2ftl out.csv

# Custom tree
cargo run --manifest-path tools/ftl-csv-convert/Cargo.toml -- \
  --locales-dir path/to/locales csv2ftl translations.csv
```

## just-keep-chasing full CSV

```bash
mkdir -p tools/ftl-csv-convert/fixtures
curl -fsSL -o tools/ftl-csv-convert/fixtures/translations.csv \
  https://raw.githubusercontent.com/mlm-games/just-keep-chasing/main/locale/translations.csv

# Import into a scratch locales dir
cargo run --manifest-path tools/ftl-csv-convert/Cargo.toml -- \
  --locales-dir /tmp/jkc-locales \
  csv2ftl tools/ftl-csv-convert/fixtures/translations.csv

# Tests (sample always; full CSV if present)
cargo test --manifest-path tools/ftl-csv-convert/Cargo.toml
```

## Behaviour

| | |
|--|--|
| **Export** | All `*/main.ftl`; flat `key = value` only; `#` comments → context column; locales A–Z; keys A–Z; missing → empty cell |
| **Import** | Headers `key` **or** `STRING ID`; context `context`/`DESCRIPTION`/…; regenerate each `main.ftl`; empty cell omits key for that locale |
| **Placeholders** | `{0}`, `{ $name }`, etc. stored as plain text (no Fluent eval) |
| **Skipped** | Terms (`-x`), attributes (`.x`), junk lines (warned) |
| **CSV** | UTF-8, comma, RFC4180 quoting as needed; BOM stripped on read; no BOM on write |

## Layout

```
tools/ftl-csv-convert/src/{main,ftl,csv_io}.rs
assets/locales/{en,es,fr}/main.ftl   # samples
```
