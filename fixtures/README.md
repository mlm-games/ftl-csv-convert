# Fixtures

## `polyglot_sample.csv`

Small extract from
[mlm-games/just-keep-chasing](https://github.com/mlm-games/just-keep-chasing)
`locale/translations.csv` — Godot polyglot headers, quoted fields, `{0}` placeholders,
CJK / RTL locales.

## Full game CSV (optional integration test)

```bash
curl -fsSL -o tools/ftl-csv-convert/fixtures/translations.csv \
  https://raw.githubusercontent.com/mlm-games/just-keep-chasing/main/locale/translations.csv
```

Then:

```bash
cargo test --manifest-path tools/ftl-csv-convert/Cargo.toml
```

`full_translations_csv_if_present` runs only when that file exists.
