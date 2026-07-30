use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn bin() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_ftl-csv-convert"));
    c.env_remove("CARGO_TARGET_DIR");
    c
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

#[test]
fn polyglot_sample_csv2ftl_then_ftl2csv() {
    let tmp = tempfile::tempdir().unwrap();
    let locales = tmp.path().join("locales");
    let sample = fixtures_dir().join("polyglot_sample.csv");
    assert!(sample.is_file(), "missing fixtures/polyglot_sample.csv");

    let status = bin()
        .args([
            "--locales-dir",
            locales.to_str().unwrap(),
            "csv2ftl",
            sample.to_str().unwrap(),
        ])
        .status()
        .expect("run csv2ftl");
    assert!(status.success());

    let en = fs::read_to_string(locales.join("en/main.ftl")).unwrap();
    assert!(en.contains("GAME_TITLE = Just Keep Chasing"));
    assert!(
        en.contains("ERROR_DEVICE_NOT_FOUND = No {0} detected."),
        "Godot {{0}} placeholder must survive:\n{en}"
    );
    assert!(
        en.contains("# Use {0} in place of [device]") || en.contains("Use {0}")
    );

    assert!(locales.join("fr/main.ftl").is_file());

    let out_csv = tmp.path().join("out.csv");
    let status = bin()
        .args([
            "--locales-dir",
            locales.to_str().unwrap(),
            "ftl2csv",
            out_csv.to_str().unwrap(),
        ])
        .status()
        .expect("run ftl2csv");
    assert!(status.success());

    let csv = fs::read_to_string(&out_csv).unwrap();
    assert!(csv.contains("GAME_TITLE"));
    assert!(csv.contains("No {0} detected."));
    assert!(csv.contains("Just Keep Chasing"));
}

#[test]
fn full_translations_csv_if_present() {
    let full = fixtures_dir().join("translations.csv");
    if !full.is_file() {
        eprintln!("skip: fixtures/translations.csv not present (curl it to enable)");
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let locales = tmp.path().join("locales");

    let status = bin()
        .args([
            "--locales-dir",
            locales.to_str().unwrap(),
            "csv2ftl",
            full.to_str().unwrap(),
        ])
        .status()
        .expect("run csv2ftl full");
    assert!(status.success());

    let en = fs::read_to_string(locales.join("en/main.ftl")).unwrap();
    assert!(en.contains("GAME_TITLE"));
    assert!(locales.join("ja/main.ftl").is_file());
    assert!(locales.join("ar/main.ftl").is_file());

    let out_csv = tmp.path().join("out.csv");
    let status = bin()
        .args([
            "--locales-dir",
            locales.to_str().unwrap(),
            "--polyglot",
            "ftl2csv",
            out_csv.to_str().unwrap(),
        ])
        .status()
        .expect("run ftl2csv full");
    assert!(status.success());

    let meta = fs::metadata(&out_csv).unwrap();
    assert!(meta.len() > 10_000, "re-export unexpectedly small");
}
