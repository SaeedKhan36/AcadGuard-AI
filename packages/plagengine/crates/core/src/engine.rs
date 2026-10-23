//! Orchestration: prepare documents for ingest, and run a check
//! (normalize → candidates → align) producing a raw, persistable
//! [`CheckResult`]. Filters and attribution happen later in [`crate::report`].

use crate::align::{align_indexed, AlignParams, QueryIndex, TokenPair};
use crate::candidates::{select_candidates, CandidateParams, PostingIndex};
use crate::fingerprint::{index_fingerprints, kgram_hashes};
use crate::normalize::normalize;
use crate::store::{Store, StoreConfig};
use crate::types::{Collection, DocFormat, ExtractedDoc, Span};
use crate::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;
use xxhash_rust::xxh3::xxh3_64;

/// A document ready to be written to the store.
#[derive(Clone, Debug)]
pub struct PreparedDoc {
    pub title: String,
    pub path: String,
    pub url: Option<String>,
    pub collection: Collection,
    pub format: DocFormat,
    pub doc: ExtractedDoc,
    /// Distinct winnowed fingerprints.
    pub fingerprints: Vec<u64>,
    pub content_hash: u64,
    pub tokens: usize,
}

/// Optional sidecar `<file>.meta.json` with `{"title": ..., "url": ...}`.
#[derive(Debug, Default, Deserialize)]
struct Sidecar {
    title: Option<String>,
    url: Option<String>,
}

fn file_stem(path: &str) -> String {
    Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or(path).to_string()
}

/// Normalize + fingerprint an extracted document.
pub fn prepare(doc: ExtractedDoc, path: &str, collection: Collection, cfg: StoreConfig) -> PreparedDoc {
    let norm = normalize(&doc.text, &doc.hidden_spans());
    let hashes = norm.token_hashes();
    let fingerprints = index_fingerprints(&hashes, cfg.k, cfg.w);
    let title = doc.title.clone().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| file_stem(path));
    PreparedDoc {
        title,
        path: path.to_string(),
        url: None,
        collection,
        format: DocFormat::from_path(Path::new(path)).unwrap_or(DocFormat::Txt),
        content_hash: xxh3_64(doc.text.as_bytes()),
        tokens: hashes.len(),
        doc,
        fingerprints,
    }
}

/// Extract + prepare a file, honoring an optional `.meta.json` sidecar.
pub fn prepare_file(path: &Path, collection: Collection, cfg: StoreConfig) -> Result<PreparedDoc> {
    let doc = crate::extract::extract_file(path)?;
    let mut p = prepare(doc, &path.to_string_lossy(), collection, cfg);
    let sidecar = path.with_extension(format!("{}.meta.json", path.extension().and_then(|e| e.to_str()).unwrap_or("")));
    if let Ok(bytes) = std::fs::read(&sidecar) {
        let s: Sidecar = serde_json::from_slice(&bytes)?;
        if let Some(t) = s.title {
            p.title = t;
        }
        p.url = s.url;
    }
    Ok(p)
}

#[derive(Clone, Debug, Default)]
pub struct CheckOptions {
    pub candidates: CandidateParams,
    pub align: AlignParams,
    /// Never match against these stored documents.
    pub exclude_doc_ids: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceInfo {
    pub id: u32,
    pub title: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub collection: Collection,
}

/// One aligned passage between the submission and a source (unfiltered).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Passage {
    pub source_id: u32,
    pub sub: Span,
    pub src: Span,
    pub matched_tokens: usize,
    pub pairs: Vec<TokenPair>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubmissionInfo {
    pub title: String,
    pub path: String,
    pub format: DocFormat,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CheckStats {
    pub candidates: usize,
    pub sources_with_matches: usize,
    pub elapsed_ms: u64,
}

/// Raw detection output. Everything a report needs, before filters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CheckResult {
    pub engine: String,
    pub check_id: String,
    pub created_unix: u64,
    pub submission: SubmissionInfo,
    pub doc: ExtractedDoc,
    pub sources: Vec<SourceInfo>,
    pub passages: Vec<Passage>,
    pub stats: CheckStats,
}

impl CheckResult {
    pub fn source(&self, id: u32) -> Option<&SourceInfo> {
        self.sources.iter().find(|s| s.id == id)
    }
}

/// Run a check against the on-disk store.
pub fn check(store: &Store, doc: ExtractedDoc, submission: SubmissionInfo, opts: &CheckOptions) -> Result<CheckResult> {
    let k = store.config().k;
    check_with_index(store, k, |id| {
        let meta = store.doc_meta(id)?;
        let doc = store.doc(id)?;
        Ok(meta.zip(doc).map(|(m, d)| {
            (SourceInfo { id, title: m.title, path: m.path, url: m.url, collection: m.collection }, d)
        }))
    }, doc, submission, opts)
}

/// Run a check against any index; `load` fetches a source document by id.
pub fn check_with_index<I, L>(
    index: &I,
    k: usize,
    load: L,
    doc: ExtractedDoc,
    submission: SubmissionInfo,
    opts: &CheckOptions,
) -> Result<CheckResult>
where
    I: PostingIndex,
    L: Fn(u32) -> Result<Option<(SourceInfo, ExtractedDoc)>> + Sync,
{
    let started = std::time::Instant::now();
    let norm = normalize(&doc.text, &doc.hidden_spans());
    let token_hashes = norm.token_hashes();
    let kgrams = kgram_hashes(&token_hashes, k);
    let candidates = select_candidates(index, &kgrams, &opts.candidates, &opts.exclude_doc_ids)?;
    let qindex = QueryIndex::new(&token_hashes, opts.align.k, opts.align.max_occ);

    type Aligned = Result<Option<(SourceInfo, Vec<Passage>)>>;
    let aligned: Vec<Aligned> = candidates
        .par_iter()
        .map(|c| {
            let Some((info, src_doc)) = load(c.doc_id)? else { return Ok(None) };
            let src_norm = normalize(&src_doc.text, &src_doc.hidden_spans());
            let passages: Vec<Passage> = align_indexed(&qindex, &norm, &src_norm, &opts.align)
                .into_iter()
                .map(|a| Passage { source_id: c.doc_id, sub: a.sub, src: a.src, matched_tokens: a.matched_tokens, pairs: a.pairs })
                .collect();
            Ok((!passages.is_empty()).then_some((info, passages)))
        })
        .collect();

    let mut sources = Vec::new();
    let mut passages = Vec::new();
    for r in aligned {
        if let Some((info, ps)) = r? {
            sources.push(info);
            passages.extend(ps);
        }
    }
    passages.sort_by_key(|p| (p.sub.start, p.source_id));
    sources.sort_by_key(|s| s.id);

    let created = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let check_id = format!("{:016x}", xxh3_64(doc.text.as_bytes()) ^ (created.as_nanos() as u64));
    Ok(CheckResult {
        engine: format!("plagengine {}", env!("CARGO_PKG_VERSION")),
        check_id,
        created_unix: created.as_secs(),
        submission,
        stats: CheckStats {
            candidates: candidates.len(),
            sources_with_matches: sources.len(),
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
        doc,
        sources,
        passages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidates::MemIndex;

    #[test]
    fn check_finds_passages_in_memory() {
        let cfg = StoreConfig::default();
        let src = "The industrial revolution transformed manufacturing processes across Europe and North America \
                   during the eighteenth and nineteenth centuries, replacing hand production with machines.";
        let p = prepare(ExtractedDoc::plain(src), "src.txt", Collection::Publications, cfg);
        let mut idx = MemIndex::default();
        idx.insert_doc(7, &p.fingerprints);
        let sub = format!("In this essay I argue many things. {src} That is all I have to say.");
        let info = SourceInfo { id: 7, title: "src".into(), path: "src.txt".into(), url: None, collection: Collection::Publications };
        let r = check_with_index(
            &idx,
            cfg.k,
            |id| Ok((id == 7).then(|| (info.clone(), ExtractedDoc::plain(src)))),
            ExtractedDoc::plain(sub.clone()),
            SubmissionInfo { title: "essay".into(), path: "essay.txt".into(), format: DocFormat::Txt },
            &CheckOptions::default(),
        )
        .unwrap();
        assert_eq!(r.passages.len(), 1);
        assert_eq!(r.passages[0].src, Span::new(0, src.chars().count() - 1)); // trailing '.' is not a token
        assert_eq!(r.sources[0].id, 7);
        // Round-trips through JSON.
        let back: CheckResult = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(back, r);
    }
}
