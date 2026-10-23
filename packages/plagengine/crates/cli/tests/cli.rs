//! End-to-end tests of the `plagengine` binary.

use plagengine_core::extract::docx::{DocxBuilder, RunSpec};
use plagengine_core::extract::testing::minimal_pdf;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const WEB: &str = "The printing press was invented by Johannes Gutenberg around 1440 and it rapidly transformed \
    the spread of information across Europe, enabling mass literacy and the scientific revolution that followed.";
const PUB_LINES: [&str; 3] = [
    "Antibiotic resistance arises when bacteria evolve mechanisms that",
    "neutralize the drugs designed to kill them, making common infections",
    "increasingly difficult to treat in hospitals around the world.",
];
const OWN: &str = "This paragraph is entirely my own writing about a topic I care deeply about and want to explore further. ";

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_plagengine"))
}

fn run(db: &Path, args: &[&str]) -> String {
    let out = bin().arg("--db").arg(db).args(args).output().expect("run plagengine");
    assert!(out.status.success(), "plagengine {args:?} failed:\n{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

fn json(db: &Path, args: &[&str]) -> Value {
    serde_json::from_str(&run(db, args)).expect("valid JSON")
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    db: PathBuf,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("web")).unwrap();
    std::fs::create_dir_all(root.join("pubs")).unwrap();
    std::fs::write(root.join("web/press.txt"), WEB).unwrap();
    std::fs::write(root.join("web/press.txt.meta.json"), r#"{"title":"History of printing","url":"https://example.org/press"}"#).unwrap();
    std::fs::write(root.join("pubs/amr.pdf"), minimal_pdf(&PUB_LINES)).unwrap();
    let db = root.join("index.db");
    run(&db, &["ingest", root.join("web").to_str().unwrap(), "--collection", "internet"]);
    run(&db, &["ingest", root.join("pubs").to_str().unwrap(), "--collection", "publications"]);
    Fixture { _dir: dir, root, db }
}

/// A .docx essay: copied web text with homoglyphs and white "glue" letters,
/// a quoted+cited copy of the publication, and enough own text for flags.
fn essay(root: &Path) -> PathBuf {
    let mut copied = Vec::new();
    for (i, w) in WEB.split(' ').enumerate() {
        if i > 0 {
            copied.push(if i % 3 == 0 { RunSpec::white("x") } else { RunSpec::plain(" ") });
        }
        // Cyrillic 'о' in "spread"/"Europe" etc.
        copied.push(RunSpec::plain(w.replace("information", "infоrmation")));
    }
    let bytes = DocxBuilder::default()
        .title("My essay")
        .paragraph(vec![RunSpec::plain(OWN.repeat(3))])
        .paragraph(copied)
        .paragraph(vec![RunSpec::plain(format!("As researchers explain, “{}” (Lee, 2021).", PUB_LINES.join(" ")))])
        .paragraph(vec![RunSpec::plain(OWN.repeat(3)), RunSpec::vanish(" hidden filler words nobody sees ")])
        .build();
    let p = root.join("essay.docx");
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
fn ingest_check_report_and_filters() {
    let fx = fixture();
    let stats = json(&fx.db, &["stats"]);
    assert_eq!(stats["internet"], 1);
    assert_eq!(stats["publications"], 1);

    let essay = essay(&fx.root);
    let r = json(&fx.db, &["check", essay.to_str().unwrap(), "--store"]);
    assert_eq!(r["submission"]["title"], "My essay");
    let sim = r["similarity"]["exact"].as_f64().unwrap();
    assert!(sim > 20.0 && sim < 80.0, "similarity {sim}");

    let sources = r["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2, "{sources:#?}");
    let web = sources.iter().find(|s| s["collection"] == "internet").unwrap();
    assert_eq!(web["title"], "History of printing");
    assert_eq!(web["url"], "https://example.org/press");
    let pubs = sources.iter().find(|s| s["collection"] == "publications").unwrap();

    // Match groups: copied web text is uncited; the publication is quoted and cited.
    let matches = r["matches"].as_array().unwrap();
    let group_for = |sid: &Value| matches.iter().find(|m| &m["source_id"] == sid).unwrap()["group"].clone();
    assert_eq!(group_for(&web["source_id"]), "Not Cited or Quoted");
    assert_eq!(group_for(&pubs["source_id"]), "Cited and Quoted");
    for m in matches {
        let sr = &m["submission_range"];
        assert!(sr["end"].as_u64() > sr["start"].as_u64());
    }

    // Flags: hidden text (white glue + vanish) and replaced characters.
    let flags = &r["flags"];
    assert_eq!(flags["eligible"], true);
    assert!(flags["hidden_text"]["count"].as_u64().unwrap() >= 10);
    assert!(flags["replaced_characters"]["count"].as_u64().unwrap() >= 1);
    assert_eq!(flags["other_manipulation"]["count"], 0);

    // Re-filter by check id without re-checking.
    let id = r["check_id"].as_str().unwrap();
    let only_pubs = json(&fx.db, &["report", id, "--exclude-collection", "internet"]);
    assert!(only_pubs["sources"].as_array().unwrap().iter().all(|s| s["collection"] == "publications"));
    let no_quotes = json(&fx.db, &["report", id, "--exclude-quotes"]);
    assert!(no_quotes["sources"].as_array().unwrap().iter().all(|s| s["collection"] == "internet"));
    let big_only = json(&fx.db, &["report", id, "--min-words", "200"]);
    assert_eq!(big_only["similarity"]["matched_chars"], 0);

    // --store persisted the essay into the submissions repository.
    let stats = json(&fx.db, &["stats"]);
    assert_eq!(stats["submissions"], 1);
    let again = json(&fx.db, &["check", essay.to_str().unwrap()]);
    let top = &again["sources"][0];
    assert_eq!(top["collection"], "submissions");
    assert!(again["similarity"]["exact"].as_f64().unwrap() > 95.0);
}

#[test]
fn embedded_raw_report_is_refilterable_from_file() {
    let fx = fixture();
    let md = fx.root.join("notes.md");
    std::fs::write(
        &md,
        format!("# Notes\n\n{OWN}\n\n> {WEB}\n\n{OWN}\n"),
    )
    .unwrap();
    let out = fx.root.join("report.json");
    run(&fx.db, &["check", md.to_str().unwrap(), "--embed-raw", "-o", out.to_str().unwrap()]);
    let r: Value = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    assert!(r["raw"].is_object());
    assert!(r["similarity"]["exact"].as_f64().unwrap() > 30.0);
    // The block quote makes it "Missing Citation" (quoted, no citation).
    assert_eq!(r["matches"][0]["group"], "Missing Citation");
    let filtered = json(&fx.db, &["report", out.to_str().unwrap(), "--exclude-quotes"]);
    assert_eq!(filtered["similarity"]["matched_chars"], 0);
    assert!(filtered["excluded"]["excluded_chars"].as_u64().unwrap() > 0);
}

#[test]
fn errors_are_reported() {
    let fx = fixture();
    let out = bin().arg("--db").arg(&fx.db).args(["check", "missing.docx"]).output().unwrap();
    assert!(!out.status.success());
    let out = bin().arg("--db").arg(&fx.db).args(["report", "nope"]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("no saved check"));
    let out = bin().args(["ingest", ".", "--collection", "bogus"]).output().unwrap();
    assert!(!out.status.success());
}
