//! Adversarial evaluation set.
//!
//! From a directory of plain-text documents, a seeded generator builds
//! suspicious documents: own text (from documents that are never indexed)
//! around one passage copied from an indexed source and disguised by an
//! attack. Ground truth is exact, so PAN metrics apply; integrity flags are
//! checked too (and must stay silent on clean copies).

use super::{evaluate, Case, Detection, Metrics, Rng};
use crate::engine::{check, prepare, CheckOptions, SubmissionInfo};
use crate::extract::docx::{extract_docx, DocxBuilder, RunSpec};
use crate::extract::{decode_text, extract_file};
use crate::report::{build_report, Filters};
use crate::store::{Store, StoreConfig};
use crate::types::{Collection, DocFormat, ExtractedDoc, Span};
use crate::{Error, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attack {
    /// Verbatim copy (control).
    Clean,
    /// Latin letters replaced by Cyrillic/Greek look-alikes.
    Homoglyph,
    /// Zero-width / invisible characters inserted inside words.
    InvisibleChars,
    /// .docx with white letters replacing spaces plus hidden filler text.
    HiddenText,
    /// Synonym substitution, small deletions, local swaps.
    LightReword,
    /// Rewording + homoglyphs + invisible characters together.
    Combined,
}

impl Attack {
    pub const ALL: [Attack; 6] =
        [Attack::Clean, Attack::Homoglyph, Attack::InvisibleChars, Attack::HiddenText, Attack::LightReword, Attack::Combined];

    fn name(&self) -> &'static str {
        match self {
            Attack::Clean => "clean",
            Attack::Homoglyph => "homoglyph",
            Attack::InvisibleChars => "invisible_chars",
            Attack::HiddenText => "hidden_text",
            Attack::LightReword => "light_reword",
            Attack::Combined => "combined",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AdvOptions {
    /// Directory of .txt documents (e.g. PAN source-document/).
    pub corpus: PathBuf,
    pub out_dir: PathBuf,
    /// Documents indexed as sources.
    pub sources: usize,
    pub cases_per_attack: usize,
    pub seed: u64,
    pub check: CheckOptions,
    pub store: StoreConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeneratedCase {
    pub file: String,
    pub attack: Attack,
    pub case: Case,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct AttackResult {
    /// Raw aligned passages (every candidate source) scored PAN-style.
    pub metrics: Metrics,
    /// What the user sees: report matches after attribution.
    pub report_metrics: Metrics,
    /// Mean Turnitin-style similarity % of the generated documents.
    pub mean_similarity: f64,
    /// Which flag is expected to fire, if any.
    pub expected_flags: Vec<&'static str>,
    /// Documents where every expected flag fired.
    pub flagged_documents: usize,
    /// Documents where an unexpected flag fired.
    pub unexpected_flag_documents: usize,
    pub documents: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdvResult {
    pub out_dir: String,
    pub sources_indexed: usize,
    pub overall: Metrics,
    pub overall_report: Metrics,
    pub by_attack: BTreeMap<String, AttackResult>,
}

// ---------------------------------------------------------------- attacks

fn cyrillic_for(c: char) -> Option<char> {
    Some(match c {
        'a' => 'а', 'c' => 'с', 'e' => 'е', 'o' => 'о', 'p' => 'р', 'x' => 'х', 'y' => 'у', 'i' => 'і', 'j' => 'ј',
        's' => 'ѕ', 'A' => 'А', 'B' => 'В', 'C' => 'С', 'E' => 'Е', 'H' => 'Н', 'I' => 'І', 'J' => 'Ј', 'K' => 'К',
        'M' => 'М', 'O' => 'О', 'P' => 'Р', 'S' => 'Ѕ', 'T' => 'Т', 'X' => 'Х', 'Y' => 'У',
        _ => return None,
    })
}

pub fn homoglyph_attack(text: &str, rng: &mut Rng, p: f64) -> String {
    text.chars().map(|c| match cyrillic_for(c) {
        Some(r) if rng.chance(p) => r,
        _ => c,
    }).collect()
}

pub fn invisible_attack(text: &str, rng: &mut Rng, p: f64) -> String {
    const INV: [char; 5] = ['\u{200B}', '\u{200C}', '\u{200D}', '\u{2060}', '\u{FEFF}'];
    let mut out = String::with_capacity(text.len() * 2);
    for word in text.split_inclusive(char::is_whitespace) {
        let chars: Vec<char> = word.chars().collect();
        let letters = chars.iter().filter(|c| c.is_alphabetic()).count();
        if letters >= 4 && rng.chance(p) {
            let at = 1 + rng.below(letters - 2);
            let mut seen = 0;
            for &c in &chars {
                out.push(c);
                if c.is_alphabetic() {
                    seen += 1;
                    if seen == at {
                        out.push(INV[rng.below(INV.len())]);
                    }
                }
            }
        } else {
            out.push_str(word);
        }
    }
    out
}

const SYNONYMS: &[(&str, &str)] = &[
    ("said", "stated"), ("great", "large"), ("small", "little"), ("large", "big"), ("big", "large"), ("began", "started"),
    ("begin", "start"), ("came", "arrived"), ("saw", "noticed"), ("very", "extremely"), ("many", "numerous"),
    ("often", "frequently"), ("quickly", "rapidly"), ("answered", "replied"), ("asked", "inquired"), ("old", "aged"),
    ("house", "home"), ("little", "small"), ("good", "fine"), ("man", "person"), ("men", "people"), ("went", "proceeded"),
    ("looked", "gazed"), ("think", "believe"), ("thought", "believed"), ("found", "discovered"), ("almost", "nearly"),
    ("however", "nevertheless"), ("also", "additionally"), ("because", "since"), ("while", "whilst"), ("among", "amongst"),
    ("whole", "entire"), ("happy", "glad"), ("seemed", "appeared"), ("seem", "appear"), ("show", "demonstrate"),
    ("help", "assist"), ("use", "employ"), ("end", "finish"), ("children", "kids"), ("woman", "lady"), ("women", "ladies"),
    ("road", "path"), ("world", "earth"), ("country", "nation"), ("certain", "sure"), ("perhaps", "maybe"), ("quite", "rather"),
    ("enough", "sufficient"), ("place", "location"), ("way", "manner"), ("day", "daytime"), ("night", "evening"),
    ("eyes", "gaze"), ("face", "visage"), ("hand", "palm"), ("heart", "soul"), ("long", "lengthy"), ("young", "youthful"),
    ("strange", "odd"), ("beautiful", "lovely"), ("dark", "dim"), ("light", "glow"), ("knew", "understood"),
    ("know", "understand"), ("told", "informed"), ("tell", "inform"), ("took", "grabbed"), ("gave", "handed"),
    ("left", "departed"), ("felt", "sensed"), ("called", "named"), ("turned", "rotated"), ("again", "once more"),
    ("never", "not ever"), ("always", "constantly"), ("soon", "shortly"), ("suddenly", "abruptly"), ("still", "yet"),
    ("between", "amid"), ("against", "versus"), ("towards", "toward"), ("upon", "on"), ("nothing", "naught"),
    ("work", "labour"), ("money", "funds"), ("friend", "companion"), ("people", "folk"), ("power", "strength"),
    ("water", "liquid"), ("land", "ground"), ("city", "town"), ("story", "tale"), ("idea", "notion"), ("fear", "dread"),
];

fn synonym(w: &str) -> Option<&'static str> {
    let lower = w.to_lowercase();
    SYNONYMS.iter().find(|(a, _)| *a == lower).map(|(_, b)| *b)
}

pub fn reword_attack(text: &str, rng: &mut Rng) -> String {
    const DROPPABLE: &[&str] = &["very", "really", "quite", "just", "then", "so", "indeed"];
    let mut words: Vec<String> = Vec::new();
    for raw in text.split_whitespace() {
        let core: String = raw.chars().filter(|c| c.is_alphanumeric()).collect();
        let lower = core.to_lowercase();
        if DROPPABLE.contains(&lower.as_str()) && rng.chance(0.5) && raw.chars().all(|c| c.is_alphanumeric()) {
            continue;
        }
        if let Some(s) = synonym(&core).filter(|_| rng.chance(0.7)) {
            let mut rep = s.to_string();
            if core.chars().next().is_some_and(char::is_uppercase) {
                rep = rep[..1].to_uppercase() + &rep[1..];
            }
            words.push(raw.replacen(&core, &rep, 1));
            continue;
        }
        words.push(raw.to_string());
    }
    // A few local swaps of adjacent purely alphabetic words.
    for i in 0..words.len().saturating_sub(1) {
        if rng.chance(0.02) && words[i].chars().all(char::is_alphabetic) && words[i + 1].chars().all(char::is_alphabetic) {
            words.swap(i, i + 1);
        }
    }
    words.join(" ")
}

// -------------------------------------------------------------- generation

fn snap_to_word(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && !chars[i].is_whitespace() {
        i += 1;
    }
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

/// Random word-aligned chunk of `len` chars (approx). Returns (span, text).
fn chunk(chars: &[char], rng: &mut Rng, min: usize, max: usize) -> Option<(Span, String)> {
    if chars.len() < max + 10 {
        return None;
    }
    let len = min + rng.below(max - min + 1);
    let start = snap_to_word(chars, rng.below(chars.len() - len));
    let mut end = (start + len).min(chars.len());
    while end > start && !chars[end - 1].is_alphanumeric() && chars[end - 1] != '.' {
        end -= 1;
    }
    while end < chars.len() && chars[end].is_alphanumeric() {
        end += 1;
    }
    (end > start + min / 2).then(|| (Span::new(start, end), chars[start..end].iter().collect()))
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

struct Built {
    file: PathBuf,
    susp_span: Span,
}

fn build_hidden_docx(path: &Path, before: &str, passage: &str, after: &str, filler: &str, rng: &mut Rng) -> Result<Built> {
    let mut runs = Vec::new();
    let filler_words: Vec<&str> = filler.split_whitespace().collect();
    let words: Vec<&str> = passage.split(' ').collect();
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            if rng.chance(0.7) {
                let junk = ["x", "q", "z", "k", "v"][rng.below(5)];
                runs.push(RunSpec::white(junk));
            } else {
                runs.push(RunSpec::plain(" "));
            }
        }
        runs.push(RunSpec::plain(*w));
        if w.ends_with('.') && i + 1 < words.len() && rng.chance(0.4) && !filler_words.is_empty() {
            let s = rng.below(filler_words.len().saturating_sub(12).max(1));
            let text = format!(" {} ", filler_words[s..(s + 12).min(filler_words.len())].join(" "));
            runs.push(if rng.chance(0.5) { RunSpec::vanish(text) } else { RunSpec::tiny(text) });
        }
    }
    let bytes = DocxBuilder::default()
        .paragraph(vec![RunSpec::plain(before)])
        .paragraph(runs)
        .paragraph(vec![RunSpec::plain(after)])
        .build();
    std::fs::write(path, &bytes).map_err(|e| Error::Io { path: path.display().to_string(), source: e })?;
    let doc = extract_docx(&bytes)?;
    let start = before.chars().count() + 1;
    let total = doc.text.chars().count();
    let end = total - after.chars().count() - 2;
    Ok(Built { file: path.to_path_buf(), susp_span: Span::new(start, end) })
}

/// Generate the adversarial set. Returns (indexed source paths, cases).
pub fn generate(opts: &AdvOptions) -> Result<(Vec<PathBuf>, Vec<GeneratedCase>)> {
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(&opts.corpus)
        .into_iter()
        .flatten()
        .map(|e| e.path().to_path_buf())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    files.sort();
    let mut rng = Rng::new(opts.seed);
    rng.shuffle(&mut files);
    let load = |p: &Path| -> Vec<char> { std::fs::read(p).map(|b| decode_text(&b).chars().collect()).unwrap_or_default() };
    // Pick sources/fillers that are long enough.
    let mut sources = Vec::new();
    let mut fillers = Vec::new();
    for f in files {
        if sources.len() >= opts.sources && fillers.len() >= opts.sources {
            break;
        }
        let len = std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
        if !(20_000..3_000_000).contains(&len) {
            continue;
        }
        if sources.len() < opts.sources {
            sources.push(f);
        } else {
            fillers.push(f);
        }
    }
    if sources.is_empty() || fillers.is_empty() {
        return Err(Error::Invalid(format!("not enough .txt documents (20KB-3MB) under {}", opts.corpus.display())));
    }
    let docs_dir = opts.out_dir.join("docs");
    std::fs::create_dir_all(&docs_dir).map_err(|e| Error::Io { path: docs_dir.display().to_string(), source: e })?;

    let mut cases = Vec::new();
    for attack in Attack::ALL {
        let mut made = 0;
        let mut tries = 0;
        while made < opts.cases_per_attack && tries < opts.cases_per_attack * 10 {
            tries += 1;
            let src_path = &sources[rng.below(sources.len())];
            let fill_path = &fillers[rng.below(fillers.len())];
            let (src_chars, fill_chars) = (load(src_path), load(fill_path));
            let Some((src_span, passage)) = chunk(&src_chars, &mut rng, 400, 1500) else { continue };
            let Some((_, before)) = chunk(&fill_chars, &mut rng, 1500, 3000) else { continue };
            let Some((_, after)) = chunk(&fill_chars, &mut rng, 1500, 3000) else { continue };
            let (before, after) = (collapse_ws(&before), collapse_ws(&after));
            let passage = collapse_ws(&passage);
            let name = format!("{}-{:03}", attack.name(), made);
            let built = if attack == Attack::HiddenText {
                let Some((_, filler)) = chunk(&fill_chars, &mut rng, 400, 800) else { continue };
                build_hidden_docx(&docs_dir.join(format!("{name}.docx")), &before, &passage, &after, &filler, &mut rng)?
            } else {
                let disguised = match attack {
                    Attack::Clean => passage.clone(),
                    Attack::Homoglyph => homoglyph_attack(&passage, &mut rng, 0.3),
                    Attack::InvisibleChars => invisible_attack(&passage, &mut rng, 0.6),
                    Attack::LightReword => reword_attack(&passage, &mut rng),
                    Attack::Combined => {
                        let r = reword_attack(&passage, &mut rng);
                        invisible_attack(&homoglyph_attack(&r, &mut rng, 0.2), &mut rng, 0.4)
                    }
                    Attack::HiddenText => unreachable!(),
                };
                let text = format!("{before}\n\n{disguised}\n\n{after}\n");
                let path = docs_dir.join(format!("{name}.txt"));
                std::fs::write(&path, &text).map_err(|e| Error::Io { path: path.display().to_string(), source: e })?;
                let start = before.chars().count() + 2;
                Built { file: path, susp_span: Span::new(start, start + disguised.chars().count()) }
            };
            let file_name = built.file.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            cases.push(GeneratedCase {
                file: built.file.display().to_string(),
                attack,
                case: Case {
                    susp: file_name,
                    src: src_path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                    susp_span: built.susp_span,
                    src_span,
                    kind: attack.name().to_string(),
                },
            });
            made += 1;
        }
    }
    let truth = opts.out_dir.join("truth.json");
    std::fs::write(&truth, serde_json::to_vec_pretty(&cases)?).map_err(|e| Error::Io { path: truth.display().to_string(), source: e })?;
    Ok((sources, cases))
}

fn expected_flags(a: Attack) -> Vec<&'static str> {
    match a {
        Attack::Clean | Attack::LightReword => vec![],
        Attack::Homoglyph => vec!["replaced_characters"],
        Attack::InvisibleChars => vec!["other_manipulation"],
        Attack::HiddenText => vec!["hidden_text"],
        Attack::Combined => vec!["replaced_characters", "other_manipulation"],
    }
}

/// Generate, index, check and score the adversarial set.
pub fn run(opts: &AdvOptions) -> Result<AdvResult> {
    let (sources, cases) = generate(opts)?;
    let db = opts.out_dir.join("adv-eval.db");
    if db.exists() {
        std::fs::remove_file(&db).map_err(|e| Error::Io { path: db.display().to_string(), source: e })?;
    }
    let store = Store::open_with(&db, opts.store)?;
    let prepared: Vec<_> = sources
        .par_iter()
        .filter_map(|p| {
            let text = decode_text(&std::fs::read(p).ok()?);
            Some(prepare(ExtractedDoc::plain(text), &p.display().to_string(), Collection::Internet, opts.store))
        })
        .collect();
    store.add_documents(&prepared)?;

    type Outcome = (Attack, Vec<Detection>, f64, [bool; 3], Vec<Detection>);
    let outcomes: Vec<Outcome> = cases
        .par_iter()
        .map(|gc| -> Result<Outcome> {
            let path = Path::new(&gc.file);
            let doc = extract_file(path)?;
            let format = DocFormat::from_path(path).unwrap_or(DocFormat::Txt);
            let info = SubmissionInfo { title: gc.case.susp.clone(), path: gc.file.clone(), format };
            let raw = check(&store, doc, info, &opts.check)?;
            let report = build_report(&raw, &Filters::default());
            let dets = raw
                .passages
                .iter()
                .map(|p| Detection {
                    susp: gc.case.susp.clone(),
                    src: raw.source(p.source_id).and_then(|s| Path::new(&s.path).file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_default(),
                    susp_span: p.sub,
                    src_span: p.src,
                })
                .collect();
            let f = &report.flags;
            let fired = [f.hidden_text.count > 0, f.replaced_characters.count > 0, f.other_manipulation.count > 0];
            let shown = report
                .matches
                .iter()
                .map(|m| Detection {
                    susp: gc.case.susp.clone(),
                    src: raw.source(m.source_id).and_then(|s| Path::new(&s.path).file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_default(),
                    susp_span: m.submission_range,
                    src_span: m.source_range,
                })
                .collect();
            Ok((gc.attack, dets, report.similarity.exact, fired, shown))
        })
        .collect::<Result<Vec<_>>>()?;

    let all_cases: Vec<Case> = cases.iter().map(|c| c.case.clone()).collect();
    let all_dets: Vec<Detection> = outcomes.iter().flat_map(|o| o.1.clone()).collect();
    let mut by_attack = BTreeMap::new();
    for attack in Attack::ALL {
        let cs: Vec<Case> = cases.iter().filter(|c| c.attack == attack).map(|c| c.case.clone()).collect();
        let os: Vec<&Outcome> = outcomes.iter().filter(|o| o.0 == attack).collect();
        let ds: Vec<Detection> = os.iter().flat_map(|o| o.1.clone()).collect();
        let shown: Vec<Detection> = os.iter().flat_map(|o| o.4.clone()).collect();
        let expected = expected_flags(attack);
        let names = ["hidden_text", "replaced_characters", "other_manipulation"];
        let flagged = os.iter().filter(|o| expected.iter().all(|e| o.3[names.iter().position(|n| n == e).unwrap()])).count();
        let unexpected = os
            .iter()
            .filter(|o| names.iter().enumerate().any(|(i, n)| o.3[i] && !expected.contains(n)))
            .count();
        by_attack.insert(
            attack.name().to_string(),
            AttackResult {
                metrics: evaluate(&cs, &ds),
                report_metrics: evaluate(&cs, &shown),
                mean_similarity: os.iter().map(|o| o.2).sum::<f64>() / os.len().max(1) as f64,
                expected_flags: expected,
                flagged_documents: flagged,
                unexpected_flag_documents: unexpected,
                documents: os.len(),
            },
        );
    }
    Ok(AdvResult {
        out_dir: opts.out_dir.display().to_string(),
        sources_indexed: prepared.len(),
        overall: evaluate(&all_cases, &all_dets),
        overall_report: evaluate(&all_cases, &outcomes.iter().flat_map(|o| o.4.clone()).collect::<Vec<_>>()),
        by_attack,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::normalize;

    #[test]
    fn attacks_preserve_normalized_text_where_expected() {
        let text = "The scientists observed a significant increase in ocean temperatures across the Pacific.";
        let mut rng = Rng::new(1);
        let h = homoglyph_attack(text, &mut rng, 0.5);
        assert_ne!(h, text);
        assert_eq!(normalize(&h, &[]).token_hashes(), normalize(text, &[]).token_hashes());
        let z = invisible_attack(text, &mut rng, 1.0);
        assert!(z.chars().count() > text.chars().count());
        assert_eq!(normalize(&z, &[]).token_hashes(), normalize(text, &[]).token_hashes());
        let r = reword_attack("He said the old house was very large and he thought it strange.", &mut Rng::new(3));
        assert_ne!(r, "He said the old house was very large and he thought it strange.");
    }

    #[test]
    fn hidden_docx_ground_truth_is_exact() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("h.docx");
        let passage = "One two three. Four five six seven.";
        let b = build_hidden_docx(&p, "Before text here.", passage, "After text.", "filler words go here and here", &mut Rng::new(5)).unwrap();
        let doc = extract_file(&p).unwrap();
        let chars: Vec<char> = doc.text.chars().collect();
        let got: String = chars[b.susp_span.start..b.susp_span.end].iter().collect();
        assert!(got.starts_with("One"), "{got:?}");
        assert!(got.ends_with("seven."), "{got:?}");
    }
}
