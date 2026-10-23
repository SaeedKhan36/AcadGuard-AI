//! plagengine command line.
#![forbid(unsafe_code)]

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use plagengine_core::engine::{prepare_file, CheckOptions, SubmissionInfo};
use plagengine_core::eval::{adversarial, pan};
use plagengine_core::store::AddOutcome;
use plagengine_core::{build_report, check, extract, Collection, DocFormat, Filters, Report, Store, StoreConfig};
use rayon::prelude::*;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "plagengine", version, about = "Plagiarism detection engine with Turnitin-style similarity reports")]
struct Cli {
    /// Path of the embedded index (created if missing).
    #[arg(long, global = true, env = "PLAGENGINE_DB", default_value = "plagengine.db")]
    db: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Add a file or directory (recursively) to a source collection.
    Ingest {
        path: PathBuf,
        #[arg(long, short)]
        collection: Collection,
        /// Documents per write transaction.
        #[arg(long, default_value_t = 500)]
        batch: usize,
    },
    /// Check a document and print the JSON similarity report.
    Check {
        file: PathBuf,
        /// Add the checked document to the `submissions` collection.
        #[arg(long)]
        store: bool,
        /// Embed the raw check in the report so it can be re-filtered from the file alone.
        #[arg(long)]
        embed_raw: bool,
        #[command(flatten)]
        filters: FilterArgs,
        #[command(flatten)]
        out: OutArgs,
        /// Maximum candidate sources to align.
        #[arg(long, default_value_t = 200)]
        max_candidates: usize,
        /// Minimum matched words for a passage to be detected at all.
        #[arg(long, default_value_t = 8)]
        min_passage_words: usize,
    },
    /// Re-apply filters to a saved check (by check id or report JSON with --embed-raw).
    Report {
        /// A check id printed in a report, or a path to a report JSON containing `raw`.
        target: String,
        #[command(flatten)]
        filters: FilterArgs,
        #[command(flatten)]
        out: OutArgs,
    },
    /// List stored documents.
    Sources {
        #[arg(long, short)]
        collection: Option<Collection>,
    },
    /// Show index statistics.
    Stats,
    /// Extract a document and print its text and formatting facts as JSON (debugging).
    Extract { file: PathBuf },
    /// Run evaluations.
    #[command(subcommand)]
    Eval(EvalCmd),
}

#[derive(Subcommand)]
enum EvalCmd {
    /// PAN-PC-11 external detection corpus: precision / recall / granularity / plagdet.
    Pan(PanArgs),
    /// Re-score a dump from `eval pan --dump` (e.g. another length floor).
    Rescore {
        dump: PathBuf,
        #[arg(long, default_value_t = 200)]
        min_detection_chars: usize,
    },
    /// Generate and evaluate the adversarial set (homoglyphs, hidden text, invisible chars, rewording).
    Adversarial(AdvArgs),
}

#[derive(Args, Clone)]
struct PanArgs {
    /// The PAN-PC-11 `external-detection-corpus` directory.
    #[arg(long)]
    corpus: PathBuf,
    /// Working directory for the evaluation index.
    #[arg(long, default_value = "eval-work/pan")]
    work_dir: PathBuf,
    /// Number of suspicious documents to evaluate (0 = all 11,093).
    #[arg(long, default_value_t = 500)]
    limit: usize,
    /// Extra unreferenced source documents to index as distractors (-1 = all).
    #[arg(long, default_value_t = 2000, allow_hyphen_values = true)]
    distractors: i64,
    #[arg(long, default_value_t = 2011)]
    seed: u64,
    /// Reuse the evaluation index from a previous run.
    #[arg(long)]
    reuse_index: bool,
    /// Write ground truth + detections JSON to this file.
    #[arg(long)]
    dump: Option<PathBuf>,
    /// Ignore detections shorter than N chars (PAN cases are ≥ ~200 chars).
    #[arg(long, default_value_t = 200)]
    min_detection_chars: usize,
    #[command(flatten)]
    tuning: TuningArgs,
}

#[derive(Args, Clone)]
struct AdvArgs {
    /// Directory of .txt documents to build the set from (e.g. PAN source-document/).
    #[arg(long)]
    corpus: PathBuf,
    #[arg(long, default_value = "eval-work/adversarial")]
    out_dir: PathBuf,
    /// Documents indexed as sources.
    #[arg(long, default_value_t = 200)]
    sources: usize,
    #[arg(long, default_value_t = 50)]
    cases_per_attack: usize,
    #[arg(long, default_value_t = 7)]
    seed: u64,
    #[command(flatten)]
    tuning: TuningArgs,
}

/// Detection parameters exposed for evaluation/tuning. Unset values use
/// the profile defaults (`eval pan`: PAN profile; `eval adversarial`: the
/// same defaults as `check`).
#[derive(Args, Clone)]
struct TuningArgs {
    #[arg(long)]
    max_candidates: Option<usize>,
    /// Minimum matched words for a passage.
    #[arg(long)]
    min_passage_words: Option<usize>,
    /// Max token gap when chaining exact fragments.
    #[arg(long)]
    max_gap: Option<usize>,
    /// Max token gap when merging passages.
    #[arg(long)]
    merge_gap: Option<usize>,
    /// Seed k-gram size for alignment.
    #[arg(long)]
    seed_k: Option<usize>,
    /// Coarse merge tier: max gap in tokens (0 disables).
    #[arg(long)]
    coarse_gap: Option<usize>,
    /// Coarse merge tier: minimum density of merged passages.
    #[arg(long)]
    coarse_density: Option<f64>,
    /// Coarse merge tier: diagonal drift always tolerated (tokens).
    #[arg(long)]
    coarse_drift: Option<usize>,
    /// Final grouping distance (tokens) for passages from the same source.
    #[arg(long)]
    final_gap: Option<usize>,
    /// Minimum densest-window fingerprint hits for a candidate source.
    #[arg(long)]
    min_hits: Option<usize>,
    /// Minimum matched tokens of the fine-tier core of a passage.
    #[arg(long)]
    min_core: Option<usize>,
}

impl TuningArgs {
    fn options(&self, mut o: CheckOptions) -> CheckOptions {
        let a = &mut o.align;
        let c = &mut o.candidates;
        self.max_candidates.inspect(|v| c.max_candidates = *v);
        self.min_hits.inspect(|v| c.min_hits = *v);
        self.min_passage_words.inspect(|v| a.min_tokens = *v);
        self.max_gap.inspect(|v| a.max_gap = *v);
        self.merge_gap.inspect(|v| a.merge_gap = *v);
        self.seed_k.inspect(|v| a.k = *v);
        self.coarse_gap.inspect(|v| a.coarse_gap = *v);
        self.coarse_density.inspect(|v| a.coarse_min_density = *v);
        self.coarse_drift.inspect(|v| a.coarse_drift = *v);
        self.final_gap.inspect(|v| a.final_gap = *v);
        self.min_core.inspect(|v| a.min_core = *v);
        o
    }
}

#[derive(Args, Clone, Default)]
struct FilterArgs {
    /// Exclude text inside quotation marks / block quotes.
    #[arg(long)]
    exclude_quotes: bool,
    /// Exclude the bibliography / references section.
    #[arg(long)]
    exclude_bibliography: bool,
    /// Exclude matches smaller than N words.
    #[arg(long, value_name = "N", default_value_t = 0)]
    min_words: usize,
    /// Exclude sources contributing less than N percent.
    #[arg(long, value_name = "N", default_value_t = 0.0)]
    min_percent: f64,
    /// Exclude a source by id (repeatable).
    #[arg(long = "exclude-source", value_name = "ID")]
    exclude_sources: Vec<u32>,
    /// Exclude a whole collection (repeatable).
    #[arg(long = "exclude-collection", value_name = "NAME")]
    exclude_collections: Vec<Collection>,
}

impl FilterArgs {
    fn to_filters(&self) -> Filters {
        Filters {
            exclude_quotes: self.exclude_quotes,
            exclude_bibliography: self.exclude_bibliography,
            min_words: self.min_words,
            min_percent: self.min_percent,
            exclude_sources: self.exclude_sources.clone(),
            exclude_collections: self.exclude_collections.clone(),
        }
    }
}

#[derive(Args, Clone, Default)]
struct OutArgs {
    /// Compact (single-line) JSON.
    #[arg(long)]
    compact: bool,
    /// Write to a file instead of stdout.
    #[arg(long, short)]
    output: Option<PathBuf>,
}

fn emit<T: serde::Serialize>(value: &T, out: &OutArgs) -> Result<()> {
    let json = if out.compact { serde_json::to_string(value)? } else { serde_json::to_string_pretty(value)? };
    match &out.output {
        Some(p) => std::fs::write(p, json + "\n").with_context(|| format!("writing {}", p.display()))?,
        None => {
            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "{json}")?;
        }
    }
    Ok(())
}

fn supported_files(root: &Path) -> Vec<PathBuf> {
    if root.is_file() {
        return vec![root.to_path_buf()];
    }
    let mut files: Vec<PathBuf> = walkdir(root)
        .into_iter()
        .filter(|p| DocFormat::from_path(p).is_some() && !p.to_string_lossy().ends_with(".meta.json"))
        .collect();
    files.sort();
    files
}

fn walkdir(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out
}

/// Buffered postings before a flush (~12 bytes each in memory).
const POSTINGS_FLUSH: usize = 40_000_000;

fn ingest(store: &Store, path: &Path, collection: Collection, batch: usize) -> Result<()> {
    let files = supported_files(path);
    if files.is_empty() {
        bail!("no .txt/.md/.pdf/.docx files under {}", path.display());
    }
    let cfg = store.config();
    let (mut added, mut dup, mut failed) = (0usize, 0usize, 0usize);
    let started = std::time::Instant::now();
    let mut writer = store.bulk_writer(POSTINGS_FLUSH);
    for chunk in files.chunks(batch.max(1)) {
        let prepared: Vec<_> = chunk.par_iter().map(|f| (f, prepare_file(f, collection, cfg))).collect();
        let mut ok = Vec::with_capacity(prepared.len());
        for (f, r) in prepared {
            match r {
                Ok(p) => ok.push(p),
                Err(e) => {
                    failed += 1;
                    eprintln!("skip {}: {e}", f.display());
                }
            }
        }
        for o in writer.add(&ok)? {
            match o {
                AddOutcome::Added(_) => added += 1,
                AddOutcome::Duplicate(_) => dup += 1,
            }
        }
        eprintln!("  {} / {} files", added + dup + failed, files.len());
    }
    writer.finish()?;
    eprintln!(
        "ingested {added} new document(s) into '{collection}' ({dup} duplicate, {failed} failed) in {:.1}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

fn load_raw(store_path: &Path, target: &str) -> Result<plagengine_core::CheckResult> {
    let p = Path::new(target);
    if p.is_file() {
        let report: Report = serde_json::from_slice(&std::fs::read(p)?).context("parsing report JSON")?;
        return report.raw.map(|b| *b).context("report has no embedded raw check; re-run `check --embed-raw` or pass a check id");
    }
    let store = Store::open(store_path)?;
    let bytes = store.load_check(target)?.with_context(|| format!("no saved check with id '{target}'"))?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Ingest { path, collection, batch } => {
            let store = Store::open(&cli.db)?;
            ingest(&store, &path, collection, batch)?;
        }
        Cmd::Check { file, store: add, embed_raw, filters, out, max_candidates, min_passage_words } => {
            let store = Store::open(&cli.db)?;
            let doc = extract::extract_file(&file)?;
            let format = DocFormat::from_path(&file).unwrap_or(DocFormat::Txt);
            let title = doc.title.clone().unwrap_or_else(|| file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
            let mut opts = CheckOptions::default();
            opts.candidates.max_candidates = max_candidates;
            opts.align.min_tokens = min_passage_words;
            let info = SubmissionInfo { title, path: file.display().to_string(), format };
            let raw = check(&store, doc, info, &opts)?;
            store.save_check(&raw.check_id, &serde_json::to_vec(&raw)?)?;
            if add {
                let p = prepare_file(&file, Collection::Submissions, store.config())?;
                match store.add_documents(&[p])?[0] {
                    AddOutcome::Added(id) => eprintln!("stored as submission #{id}"),
                    AddOutcome::Duplicate(id) => eprintln!("already stored as #{id}"),
                }
            }
            let mut report = build_report(&raw, &filters.to_filters());
            if embed_raw {
                report.raw = Some(Box::new(raw));
            }
            emit(&report, &out)?;
        }
        Cmd::Report { target, filters, out } => {
            let raw = load_raw(&cli.db, &target)?;
            emit(&build_report(&raw, &filters.to_filters()), &out)?;
        }
        Cmd::Sources { collection } => {
            let store = Store::open(&cli.db)?;
            let docs: Vec<_> = store.list_docs()?.into_iter().filter(|d| collection.is_none_or(|c| d.collection == c)).collect();
            emit(&docs, &OutArgs::default())?;
        }
        Cmd::Stats => {
            let store = Store::open(&cli.db)?;
            emit(&store.stats()?, &OutArgs::default())?;
        }
        Cmd::Extract { file } => {
            emit(&extract::extract_file(&file)?, &OutArgs::default())?;
        }
        Cmd::Eval(EvalCmd::Pan(a)) => {
            let opts = pan::PanOptions {
                corpus: a.corpus,
                work_dir: a.work_dir,
                limit: a.limit,
                distractors: if a.distractors < 0 { usize::MAX } else { a.distractors as usize },
                seed: a.seed,
                reuse_index: a.reuse_index,
                check: a.tuning.options(pan::pan_profile()),
                store: StoreConfig::default(),
                dump: a.dump,
                min_detection_chars: a.min_detection_chars,
            };
            emit(&pan::run(&opts)?, &OutArgs::default())?;
        }
        Cmd::Eval(EvalCmd::Rescore { dump, min_detection_chars }) => {
            emit(&pan::rescore(&dump, min_detection_chars)?, &OutArgs::default())?;
        }
        Cmd::Eval(EvalCmd::Adversarial(a)) => {
            let opts = adversarial::AdvOptions {
                corpus: a.corpus,
                out_dir: a.out_dir,
                sources: a.sources,
                cases_per_attack: a.cases_per_attack,
                seed: a.seed,
                check: a.tuning.options(CheckOptions::default()),
                store: StoreConfig::default(),
            };
            emit(&adversarial::run(&opts)?, &OutArgs::default())?;
        }
    }
    Ok(())
}
