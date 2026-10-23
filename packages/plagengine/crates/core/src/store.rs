//! Embedded on-disk index (redb, single file). Persists documents of every
//! collection, the inverted fingerprint index and saved check results.

use crate::candidates::PostingIndex;
use crate::engine::PreparedDoc;
use crate::types::{Collection, DocFormat, ExtractedDoc};
use crate::{Error, Result};
use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const META: TableDefinition<&str, &[u8]> = TableDefinition::new("meta");
const DOCS: TableDefinition<u32, &[u8]> = TableDefinition::new("docs");
const TEXT: TableDefinition<u32, &[u8]> = TableDefinition::new("doc_text");
const CONTENT: TableDefinition<u64, u32> = TableDefinition::new("content");
const POSTINGS: TableDefinition<(u64, u32), ()> = TableDefinition::new("postings");
const CHECKS: TableDefinition<&str, &[u8]> = TableDefinition::new("checks");

/// Index parameters, fixed when the store is created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreConfig {
    pub k: usize,
    pub w: usize,
}

impl Default for StoreConfig {
    fn default() -> Self {
        StoreConfig { k: crate::fingerprint::DEFAULT_K, w: crate::fingerprint::DEFAULT_W }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocMeta {
    pub id: u32,
    pub title: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub collection: Collection,
    pub format: DocFormat,
    pub chars: usize,
    pub tokens: usize,
    pub content_hash: u64,
    pub added_unix: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddOutcome {
    Added(u32),
    /// Identical content already stored under this id.
    Duplicate(u32),
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct StoreStats {
    pub documents: usize,
    pub internet: usize,
    pub publications: usize,
    pub submissions: usize,
    pub postings: u64,
    pub checks: u64,
    pub config: Option<StoreConfig>,
}

trait StoreErr<T> {
    fn st(self) -> Result<T>;
}
impl<T, E: std::fmt::Display> StoreErr<T> for std::result::Result<T, E> {
    fn st(self) -> Result<T> {
        self.map_err(|e| Error::Store(e.to_string()))
    }
}

pub struct Store {
    db: Database,
    config: StoreConfig,
}

fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl Store {
    /// Open an existing store or create a new one with the default config.
    pub fn open(path: impl AsRef<Path>) -> Result<Store> {
        Store::open_with(path, StoreConfig::default())
    }

    /// Open, creating with `config` when the file is new. An existing store
    /// keeps the config it was created with.
    pub fn open_with(path: impl AsRef<Path>, config: StoreConfig) -> Result<Store> {
        let db = Database::create(path.as_ref()).st()?;
        let txn = db.begin_write().st()?;
        let config = {
            let mut meta = txn.open_table(META).st()?;
            let existing = meta.get("config").st()?.map(|v| serde_json::from_slice::<StoreConfig>(v.value()));
            let cfg = match existing {
                Some(c) => c?,
                None => {
                    meta.insert("config", serde_json::to_vec(&config)?.as_slice()).st()?;
                    meta.insert("next_id", 1u32.to_le_bytes().as_slice()).st()?;
                    config
                }
            };
            txn.open_table(DOCS).st()?;
            txn.open_table(TEXT).st()?;
            txn.open_table(CONTENT).st()?;
            txn.open_table(POSTINGS).st()?;
            txn.open_table(CHECKS).st()?;
            cfg
        };
        txn.commit().st()?;
        Ok(Store { db, config })
    }

    pub fn config(&self) -> StoreConfig {
        self.config
    }

    /// Add prepared documents and index them immediately.
    pub fn add_documents(&self, docs: &[PreparedDoc]) -> Result<Vec<AddOutcome>> {
        let mut w = self.bulk_writer(usize::MAX);
        let out = w.add(docs)?;
        w.finish()?;
        Ok(out)
    }

    /// A buffered writer for large ingests: documents are written per call,
    /// postings are buffered and flushed sorted once `flush_at` are pending.
    /// Postings keys are uniformly spread, so flushing rarely (instead of per
    /// batch) avoids rewriting most of the copy-on-write B-tree each commit.
    pub fn bulk_writer(&self, flush_at: usize) -> BulkWriter<'_> {
        BulkWriter { store: self, postings: Vec::new(), content: Vec::new(), pending: HashMap::new(), flush_at }
    }

    pub fn doc_meta(&self, id: u32) -> Result<Option<DocMeta>> {
        let txn = self.db.begin_read().st()?;
        let t = txn.open_table(DOCS).st()?;
        match t.get(id).st()? {
            Some(v) => Ok(Some(serde_json::from_slice(v.value())?)),
            None => Ok(None),
        }
    }

    pub fn doc(&self, id: u32) -> Result<Option<ExtractedDoc>> {
        let txn = self.db.begin_read().st()?;
        let t = txn.open_table(TEXT).st()?;
        match t.get(id).st()? {
            Some(v) => Ok(Some(serde_json::from_slice(v.value())?)),
            None => Ok(None),
        }
    }

    pub fn list_docs(&self) -> Result<Vec<DocMeta>> {
        let txn = self.db.begin_read().st()?;
        let t = txn.open_table(DOCS).st()?;
        let mut out = Vec::new();
        for item in t.iter().st()? {
            let (_, v) = item.st()?;
            out.push(serde_json::from_slice(v.value())?);
        }
        Ok(out)
    }

    pub fn find_by_content(&self, content_hash: u64) -> Result<Option<u32>> {
        let txn = self.db.begin_read().st()?;
        let t = txn.open_table(CONTENT).st()?;
        Ok(t.get(content_hash).st()?.map(|v| v.value()))
    }

    pub fn save_check(&self, id: &str, json: &[u8]) -> Result<()> {
        let txn = self.db.begin_write().st()?;
        txn.open_table(CHECKS).st()?.insert(id, json).st()?;
        txn.commit().st()
    }

    pub fn load_check(&self, id: &str) -> Result<Option<Vec<u8>>> {
        let txn = self.db.begin_read().st()?;
        let t = txn.open_table(CHECKS).st()?;
        Ok(t.get(id).st()?.map(|v| v.value().to_vec()))
    }

    pub fn stats(&self) -> Result<StoreStats> {
        let mut s = StoreStats { config: Some(self.config), ..Default::default() };
        for m in self.list_docs()? {
            s.documents += 1;
            match m.collection {
                Collection::Internet => s.internet += 1,
                Collection::Publications => s.publications += 1,
                Collection::Submissions => s.submissions += 1,
            }
        }
        let txn = self.db.begin_read().st()?;
        s.postings = txn.open_table(POSTINGS).st()?.len().st()?;
        s.checks = txn.open_table(CHECKS).st()?.len().st()?;
        Ok(s)
    }
}

/// See [`Store::bulk_writer`]. Call [`BulkWriter::finish`] to flush and
/// observe errors; dropping flushes on a best-effort basis. Documents whose
/// postings were never flushed are invisible to checks and are re-added by
/// a later ingest (their content hash is written together with postings).
pub struct BulkWriter<'a> {
    store: &'a Store,
    postings: Vec<(u64, u32)>,
    content: Vec<(u64, u32)>,
    pending: HashMap<u64, u32>,
    flush_at: usize,
}

impl BulkWriter<'_> {
    pub fn add(&mut self, docs: &[PreparedDoc]) -> Result<Vec<AddOutcome>> {
        let txn = self.store.db.begin_write().st()?;
        let mut outcomes = Vec::with_capacity(docs.len());
        {
            let mut meta = txn.open_table(META).st()?;
            let mut next_id = meta
                .get("next_id")
                .st()?
                .map(|v| u32::from_le_bytes(v.value().try_into().unwrap_or([1, 0, 0, 0])))
                .unwrap_or(1);
            let mut docs_t = txn.open_table(DOCS).st()?;
            let mut text_t = txn.open_table(TEXT).st()?;
            let content_t = txn.open_table(CONTENT).st()?;
            for d in docs {
                let existing = match self.pending.get(&d.content_hash) {
                    Some(&id) => Some(id),
                    None => content_t.get(d.content_hash).st()?.map(|v| v.value()),
                };
                if let Some(id) = existing {
                    outcomes.push(AddOutcome::Duplicate(id));
                    continue;
                }
                let id = next_id;
                next_id += 1;
                let m = DocMeta {
                    id,
                    title: d.title.clone(),
                    path: d.path.clone(),
                    url: d.url.clone(),
                    collection: d.collection,
                    format: d.format,
                    chars: d.doc.text.chars().count(),
                    tokens: d.tokens,
                    content_hash: d.content_hash,
                    added_unix: now_unix(),
                };
                docs_t.insert(id, serde_json::to_vec(&m)?.as_slice()).st()?;
                text_t.insert(id, serde_json::to_vec(&d.doc)?.as_slice()).st()?;
                self.pending.insert(d.content_hash, id);
                self.content.push((d.content_hash, id));
                self.postings.extend(d.fingerprints.iter().map(|&h| (h, id)));
                outcomes.push(AddOutcome::Added(id));
            }
            meta.insert("next_id", next_id.to_le_bytes().as_slice()).st()?;
        }
        txn.commit().st()?;
        if self.postings.len() >= self.flush_at {
            self.flush()?;
        }
        Ok(outcomes)
    }

    /// Write buffered postings and content hashes in one transaction.
    pub fn flush(&mut self) -> Result<()> {
        if self.content.is_empty() && self.postings.is_empty() {
            return Ok(());
        }
        self.postings.sort_unstable();
        self.content.sort_unstable();
        let txn = self.store.db.begin_write().st()?;
        {
            let mut post_t = txn.open_table(POSTINGS).st()?;
            for &key in &self.postings {
                post_t.insert(key, ()).st()?;
            }
            let mut content_t = txn.open_table(CONTENT).st()?;
            for &(h, id) in &self.content {
                content_t.insert(h, id).st()?;
            }
        }
        txn.commit().st()?;
        self.postings = Vec::new();
        self.content.clear();
        self.pending.clear();
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        self.flush()
    }
}

impl Drop for BulkWriter<'_> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

impl PostingIndex for Store {
    fn lookup_many(&self, hashes: &[u64], cap: usize) -> Result<Vec<Option<Vec<u32>>>> {
        let txn = self.db.begin_read().st()?;
        let t = txn.open_table(POSTINGS).st()?;
        let mut out = Vec::with_capacity(hashes.len());
        for &h in hashes {
            let mut docs = Vec::new();
            let mut over = false;
            for item in t.range((h, 0u32)..=(h, u32::MAX)).st()? {
                let (k, _) = item.st()?;
                docs.push(k.value().1);
                if docs.len() > cap {
                    over = true;
                    break;
                }
            }
            out.push(if over { None } else { Some(docs) });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::prepare;

    #[test]
    fn roundtrip_dedupe_and_lookup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let a = prepare(ExtractedDoc::plain("one two three four five six seven eight nine ten"), "a.txt", Collection::Internet, StoreConfig::default());
        let b = prepare(ExtractedDoc::plain("alpha beta gamma delta epsilon zeta eta theta"), "b.txt", Collection::Publications, StoreConfig::default());
        {
            let store = Store::open(&path).unwrap();
            let out = store.add_documents(&[a.clone(), b.clone(), a.clone()]).unwrap();
            assert_eq!(out, vec![AddOutcome::Added(1), AddOutcome::Added(2), AddOutcome::Duplicate(1)]);
        }
        // Reopen: persisted.
        let store = Store::open_with(&path, StoreConfig { k: 9, w: 9 }).unwrap();
        assert_eq!(store.config(), StoreConfig::default(), "existing config wins");
        let m = store.doc_meta(2).unwrap().unwrap();
        assert_eq!(m.collection, Collection::Publications);
        assert_eq!(m.title, "b");
        assert_eq!(store.doc(1).unwrap().unwrap().text, a.doc.text);
        let hits = store.lookup_many(&[a.fingerprints[0], 42], 10).unwrap();
        assert_eq!(hits, vec![Some(vec![1]), Some(vec![])]);
        let s = store.stats().unwrap();
        assert_eq!((s.documents, s.internet, s.publications), (2, 1, 1));
        let out = store.add_documents(&[prepare(ExtractedDoc::plain("x y z w v u"), "c.txt", Collection::Submissions, store.config())]).unwrap();
        assert_eq!(out, vec![AddOutcome::Added(3)]);
        // Bulk writer: buffered postings become visible after finish; dedupe spans buffer.
        let d = prepare(ExtractedDoc::plain("p q r s t u v w x y"), "d.txt", Collection::Internet, store.config());
        let mut w = store.bulk_writer(1_000_000);
        assert_eq!(w.add(&[d.clone(), d.clone()]).unwrap(), vec![AddOutcome::Added(4), AddOutcome::Duplicate(4)]);
        assert_eq!(store.lookup_many(&[d.fingerprints[0]], 10).unwrap(), vec![Some(vec![])]);
        w.finish().unwrap();
        assert_eq!(store.lookup_many(&[d.fingerprints[0]], 10).unwrap(), vec![Some(vec![4])]);
        store.save_check("abc", b"{}").unwrap();
        assert_eq!(store.load_check("abc").unwrap().unwrap(), b"{}");
    }
}
