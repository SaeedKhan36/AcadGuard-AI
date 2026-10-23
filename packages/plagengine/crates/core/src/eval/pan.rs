//! PAN-PC-11 external plagiarism detection runner.
//!
//! Expects the `external-detection-corpus` directory (with
//! `suspicious-document/part*/` and `source-document/part*/`). Suspicious
//! documents are sampled deterministically; every source they reference is
//! indexed, plus `distractors` randomly chosen other sources (all of them
//! with `distractors = usize::MAX`), into a fresh on-disk store.

use super::{evaluate, evaluate_by_kind, Case, Detection, Metrics, Rng};
use crate::engine::{check, prepare_file, CheckOptions, SubmissionInfo};
use crate::store::{Store, StoreConfig};
use crate::types::{Collection, DocFormat, Span};
use crate::{Error, Result};
use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct PanOptions {
    pub corpus: PathBuf,
    pub work_dir: PathBuf,
    /// Suspicious documents to evaluate (0 = all).
    pub limit: usize,
    /// Extra non-referenced source documents to index.
    pub distractors: usize,
    pub seed: u64,
    /// Reuse an existing index in `work_dir` if present.
    pub reuse_index: bool,
    pub check: CheckOptions,
    pub store: StoreConfig,
    /// Write ground truth and detections as JSON here (debugging).
    pub dump: Option<PathBuf>,
    /// Drop detections shorter than this many chars on the suspicious side.
    /// PAN cases are long (99% ≥ 213 chars) while the corpus contains much
    /// unannotated short reuse (Bible verses, famous quotations).
    pub min_detection_chars: usize,
}

/// Metrics block shared by `run` and `rescore`.
#[derive(Clone, Debug, Serialize)]
pub struct Scores {
    pub overall: Metrics,
    pub monolingual: Metrics,
    pub by_kind: BTreeMap<String, Metrics>,
}

pub fn score(cases: &[Case], detections: &[Detection], min_detection_chars: usize) -> Scores {
    let dets: Vec<Detection> = detections.iter().filter(|d| d.susp_span.len() >= min_detection_chars).cloned().collect();
    let mono: Vec<Case> = cases.iter().filter(|c| !c.kind.starts_with("translation")).cloned().collect();
    Scores { overall: evaluate(cases, &dets), monolingual: evaluate(&mono, &dets), by_kind: evaluate_by_kind(cases, &dets) }
}

#[derive(serde::Deserialize)]
struct Dump {
    cases: Vec<Case>,
    detections: Vec<Detection>,
}

/// Re-score a dump written by `run` (with `dump`), e.g. with another
/// detection length floor, without re-running detection.
pub fn rescore(dump: &Path, min_detection_chars: usize) -> Result<Scores> {
    let bytes = std::fs::read(dump).map_err(|e| Error::Io { path: dump.display().to_string(), source: e })?;
    let d: Dump = serde_json::from_slice(&bytes)?;
    Ok(score(&d.cases, &d.detections, min_detection_chars))
}

#[derive(Clone, Debug, Serialize)]
pub struct PanTiming {
    pub ingest_seconds: f64,
    pub check_seconds: f64,
    pub suspicious_per_second: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct PanResult {
    pub corpus: String,
    pub suspicious_documents: usize,
    pub source_documents_indexed: usize,
    /// overall / monolingual (no translation cases) / per obfuscation type.
    #[serde(flatten)]
    pub scores: Scores,
    pub timing: PanTiming,
    pub parameters: serde_json::Value,
}

/// Detection parameters tuned for PAN-PC-11 (long, machine-obfuscated
/// passages; evaluation penalizes short common-phrase matches and split
/// detections): 15-word minimum, a wider coarse tier and final grouping.
pub fn pan_profile() -> CheckOptions {
    let mut o = CheckOptions::default();
    o.candidates.max_candidates = 50;
    o.align.min_tokens = 15;
    o.align.coarse_gap = 80;
    o.align.coarse_min_density = 0.15;
    o.align.final_gap = 300;
    o
}

/// Parse a PAN annotation XML into cases for suspicious document `susp`.
pub fn parse_annotations(xml: &str, susp: &str) -> Vec<Case> {
    let mut reader = Reader::from_str(xml);
    let mut out = Vec::new();
    loop {
        let ev = match reader.read_event() {
            Ok(Event::Eof) | Err(_) => break,
            Ok(ev) => ev,
        };
        let (Event::Empty(e) | Event::Start(e)) = ev else { continue };
        if e.local_name().as_ref() != "feature" {
            continue;
        }
        let attrs: HashMap<String, String> = e
            .attributes()
            .flatten()
            .filter_map(|a| {
                let k = a.key.local_name().as_ref().to_string();
                a.normalized_value(XmlVersion::Implicit1_0).ok().map(|v| (k, v.into_owned()))
            })
            .collect();
        if attrs.get("name").map(String::as_str) != Some("plagiarism") {
            continue;
        }
        let num = |k: &str| attrs.get(k).and_then(|v| v.parse::<usize>().ok());
        let (Some(to), Some(tl), Some(so), Some(sl)) = (num("this_offset"), num("this_length"), num("source_offset"), num("source_length")) else {
            continue;
        };
        let ty = attrs.get("type").cloned().unwrap_or_else(|| "unknown".into());
        let kind = match attrs.get("obfuscation") {
            Some(o) if !o.is_empty() => format!("{ty}-{o}"),
            _ => ty,
        };
        out.push(Case {
            susp: susp.to_string(),
            src: attrs.get("source_reference").cloned().unwrap_or_default(),
            susp_span: Span::new(to, to + tl),
            src_span: Span::new(so, so + sl),
            kind,
        });
    }
    out
}

fn list_txt(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(dir).into_iter().flatten() {
        let p = entry.path();
        if p.extension().is_some_and(|e| e == "txt") {
            out.push(p.to_path_buf());
        }
    }
    out.sort();
    out
}

fn name_of(p: &Path) -> String {
    p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn run(opts: &PanOptions) -> Result<PanResult> {
    let susp_dir = opts.corpus.join("suspicious-document");
    let src_dir = opts.corpus.join("source-document");
    if !susp_dir.is_dir() || !src_dir.is_dir() {
        return Err(Error::Invalid(format!(
            "{} must contain suspicious-document/ and source-document/ (point --corpus at external-detection-corpus)",
            opts.corpus.display()
        )));
    }
    let mut susp_files = list_txt(&susp_dir);
    let src_files = list_txt(&src_dir);
    let src_by_name: HashMap<String, PathBuf> = src_files.iter().map(|p| (name_of(p), p.clone())).collect();

    let mut rng = Rng::new(opts.seed);
    rng.shuffle(&mut susp_files);
    if opts.limit > 0 {
        susp_files.truncate(opts.limit);
    }
    susp_files.sort();

    // Ground truth for the sample.
    let mut cases: Vec<Case> = Vec::new();
    for f in &susp_files {
        if let Ok(xml) = std::fs::read_to_string(f.with_extension("xml")) {
            cases.extend(parse_annotations(xml.trim_start_matches('\u{FEFF}'), &name_of(f)));
        }
    }
    let mut needed: BTreeSet<String> = cases.iter().map(|c| c.src.clone()).collect();
    let mut others: Vec<&PathBuf> = src_files.iter().filter(|p| !needed.contains(&name_of(p))).collect();
    rng.shuffle(&mut others);
    for p in others.into_iter().take(opts.distractors) {
        needed.insert(name_of(p));
    }
    let to_index: Vec<PathBuf> = needed.iter().filter_map(|n| src_by_name.get(n).cloned()).collect();

    // Index.
    std::fs::create_dir_all(&opts.work_dir).map_err(|e| Error::Io { path: opts.work_dir.display().to_string(), source: e })?;
    let db_path = opts.work_dir.join("pan-eval.db");
    if !opts.reuse_index && db_path.exists() {
        std::fs::remove_file(&db_path).map_err(|e| Error::Io { path: db_path.display().to_string(), source: e })?;
    }
    let store = Store::open_with(&db_path, opts.store)?;
    let t0 = std::time::Instant::now();
    let existing: BTreeSet<String> = store.list_docs()?.into_iter().map(|d| name_of(Path::new(&d.path))).collect();
    let missing: Vec<&PathBuf> = to_index.iter().filter(|p| !existing.contains(&name_of(p))).collect();
    let mut writer = store.bulk_writer(40_000_000);
    for (i, chunk) in missing.chunks(400).enumerate() {
        let prepared: Vec<_> = chunk
            .par_iter()
            .filter_map(|p| prepare_file(p, Collection::Publications, store.config()).ok())
            .collect();
        writer.add(&prepared)?;
        eprintln!("  indexed {} / {}", (i * 400 + chunk.len()).min(missing.len()), missing.len());
    }
    writer.finish()?;
    let ingest_seconds = t0.elapsed().as_secs_f64();

    // Check.
    let t1 = std::time::Instant::now();
    let done = std::sync::atomic::AtomicUsize::new(0);
    let detections: Vec<Detection> = susp_files
        .par_iter()
        .map(|f| -> Result<Vec<Detection>> {
            let doc = crate::extract::extract_file(f)?;
            let info = SubmissionInfo { title: name_of(f), path: f.display().to_string(), format: DocFormat::Txt };
            let r = check(&store, doc, info, &opts.check)?;
            let n = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if n.is_multiple_of(100) {
                eprintln!("  checked {n} / {}", susp_files.len());
            }
            Ok(r
                .passages
                .iter()
                .map(|p| Detection {
                    susp: name_of(f),
                    src: r.source(p.source_id).map(|s| name_of(Path::new(&s.path))).unwrap_or_default(),
                    susp_span: p.sub,
                    src_span: p.src,
                })
                .collect())
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    let check_seconds = t1.elapsed().as_secs_f64();

    if let Some(path) = &opts.dump {
        let body = serde_json::json!({ "cases": cases, "detections": detections });
        std::fs::write(path, serde_json::to_vec(&body)?).map_err(|e| Error::Io { path: path.display().to_string(), source: e })?;
    }
    let parameters = serde_json::json!({
        "seed": opts.seed,
        "limit": opts.limit,
        "distractors": opts.distractors,
        "index_k": opts.store.k,
        "index_w": opts.store.w,
        "align": format!("{:?}", opts.check.align),
        "candidates": format!("{:?}", opts.check.candidates),
        "min_detection_chars": opts.min_detection_chars,
    });
    Ok(PanResult {
        corpus: opts.corpus.display().to_string(),
        suspicious_documents: susp_files.len(),
        source_documents_indexed: store.stats()?.documents,
        scores: score(&cases, &detections, opts.min_detection_chars),
        timing: PanTiming {
            ingest_seconds,
            check_seconds,
            suspicious_per_second: susp_files.len() as f64 / check_seconds.max(1e-9),
        },
        parameters,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pan_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<document reference="suspicious-document06798.txt">
  <feature name="about" authors="Voltaire" lang="en" />
  <feature name="plagiarism" type="artificial" obfuscation="low" this_language="en" this_offset="1" this_length="290" source_reference="source-document09311.txt" source_language="en" source_offset="5898" source_length="268" />
  <feature name="plagiarism" type="simulated" this_offset="500" this_length="10" source_reference="source-document00001.txt" source_offset="0" source_length="12" />
</document>"#;
        let c = parse_annotations(xml, "suspicious-document06798.txt");
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].kind, "artificial-low");
        assert_eq!(c[0].susp_span, Span::new(1, 291));
        assert_eq!(c[0].src_span, Span::new(5898, 6166));
        assert_eq!(c[1].kind, "simulated");
    }
}
