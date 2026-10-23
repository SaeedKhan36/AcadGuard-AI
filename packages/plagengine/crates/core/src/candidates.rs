//! Stage 3 — candidate retrieval from the inverted index.
//!
//! Each query k-gram hash is looked up (hash → documents). Hashes found in
//! more than `df_cap` documents are boilerplate and ignored. Documents are
//! scored by their densest cluster of hits (hits within a sliding window of
//! query positions) so that a book sharing a few scattered common phrases
//! does not outrank a source sharing one real passage.

use std::collections::HashMap;

/// Anything that can answer "which documents contain this fingerprint".
pub trait PostingIndex {
    /// For each hash, the documents containing it, or `None` when more than
    /// `cap` documents do.
    fn lookup_many(&self, hashes: &[u64], cap: usize) -> crate::Result<Vec<Option<Vec<u32>>>>;
}

/// Simple in-memory index (tests, eval scratch runs).
#[derive(Default, Debug, Clone)]
pub struct MemIndex {
    map: HashMap<u64, Vec<u32>>,
}

impl MemIndex {
    pub fn insert_doc(&mut self, doc: u32, fingerprints: &[u64]) {
        for &h in fingerprints {
            let v = self.map.entry(h).or_default();
            if v.last() != Some(&doc) {
                v.push(doc);
            }
        }
    }
}

impl PostingIndex for MemIndex {
    fn lookup_many(&self, hashes: &[u64], cap: usize) -> crate::Result<Vec<Option<Vec<u32>>>> {
        Ok(hashes
            .iter()
            .map(|h| match self.map.get(h) {
                Some(v) if v.len() > cap => None,
                Some(v) => Some(v.clone()),
                None => Some(Vec::new()),
            })
            .collect())
    }
}

#[derive(Clone, Debug)]
pub struct CandidateParams {
    pub max_candidates: usize,
    /// Minimum hits inside the best window.
    pub min_hits: usize,
    pub df_cap: usize,
    /// Window, in query k-gram positions, for the density score.
    pub window: usize,
}

impl Default for CandidateParams {
    fn default() -> Self {
        CandidateParams { max_candidates: 200, min_hits: 2, df_cap: 1000, window: 80 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub doc_id: u32,
    /// Distinct query positions with a shared fingerprint.
    pub hits: usize,
    /// Hits in the densest window.
    pub window_hits: usize,
}

/// Select candidate source documents for the query k-gram hashes.
pub fn select_candidates(
    index: &impl PostingIndex,
    query_kgrams: &[u64],
    params: &CandidateParams,
    exclude: &[u32],
) -> crate::Result<Vec<Candidate>> {
    // Look up each distinct hash once (sorted for B-tree locality).
    let mut uniq: Vec<u64> = query_kgrams.to_vec();
    uniq.sort_unstable();
    uniq.dedup();
    let postings = index.lookup_many(&uniq, params.df_cap)?;
    let lookup: HashMap<u64, &Vec<u32>> =
        uniq.iter().zip(postings.iter()).filter_map(|(h, p)| p.as_ref().filter(|v| !v.is_empty()).map(|v| (*h, v))).collect();

    let mut per_doc: HashMap<u32, Vec<u32>> = HashMap::new();
    for (pos, h) in query_kgrams.iter().enumerate() {
        if let Some(docs) = lookup.get(h) {
            for &d in docs.iter() {
                per_doc.entry(d).or_default().push(pos as u32);
            }
        }
    }

    let mut out: Vec<Candidate> = per_doc
        .into_iter()
        .filter(|(d, _)| !exclude.contains(d))
        .filter_map(|(doc_id, positions)| {
            // positions are ascending by construction
            let mut best = 0;
            let mut lo = 0;
            for hi in 0..positions.len() {
                while positions[hi] - positions[lo] >= params.window as u32 {
                    lo += 1;
                }
                best = best.max(hi - lo + 1);
            }
            (best >= params.min_hits).then_some(Candidate { doc_id, hits: positions.len(), window_hits: best })
        })
        .collect();
    out.sort_by(|a, b| b.window_hits.cmp(&a.window_hits).then(b.hits.cmp(&a.hits)).then(a.doc_id.cmp(&b.doc_id)));
    out.truncate(params.max_candidates);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fingerprint::{index_fingerprints, kgram_hashes, DEFAULT_K, DEFAULT_W};
    use crate::normalize::normalize;

    fn fp(s: &str) -> Vec<u64> {
        index_fingerprints(&normalize(s, &[]).token_hashes(), DEFAULT_K, DEFAULT_W)
    }
    fn q(s: &str) -> Vec<u64> {
        kgram_hashes(&normalize(s, &[]).token_hashes(), DEFAULT_K)
    }

    const SRC_A: &str = "Photosynthesis converts light energy into chemical energy stored in glucose molecules within the chloroplasts of plant cells.";
    const SRC_B: &str = "The French revolution began in 1789 and fundamentally transformed the political landscape of modern Europe forever.";
    const SRC_C: &str = "Unrelated text about databases indexing structures such as b-trees and log structured merge trees for storage.";

    fn index() -> MemIndex {
        let mut idx = MemIndex::default();
        idx.insert_doc(1, &fp(SRC_A));
        idx.insert_doc(2, &fp(SRC_B));
        idx.insert_doc(3, &fp(SRC_C));
        idx
    }

    #[test]
    fn finds_the_right_sources_ranked() {
        let query = format!("In my essay I note that {SRC_B} Also, {} as we know.", &SRC_A[..60]);
        let c = select_candidates(&index(), &q(&query), &CandidateParams::default(), &[]).unwrap();
        let ids: Vec<u32> = c.iter().map(|c| c.doc_id).collect();
        assert_eq!(ids, vec![2, 1]);
        assert!(c[0].hits > c[1].hits);
    }

    #[test]
    fn excludes_and_df_cap() {
        let c = select_candidates(&index(), &q(SRC_B), &CandidateParams::default(), &[2]).unwrap();
        assert!(c.is_empty());
        // A fingerprint shared by every doc is ignored when df_cap is 1.
        let mut idx = MemIndex::default();
        for d in 0..3 {
            idx.insert_doc(d, &fp(SRC_A));
        }
        let p = CandidateParams { df_cap: 1, ..Default::default() };
        assert!(select_candidates(&idx, &q(SRC_A), &p, &[]).unwrap().is_empty());
    }

    #[test]
    fn scattered_hits_do_not_qualify() {
        let mut idx = MemIndex::default();
        let fps = fp("alpha beta gamma delta epsilon zeta eta theta");
        idx.insert_doc(9, &fps[..1]);
        // A single shared hash cannot reach min_hits = 2.
        let c = select_candidates(&idx, &q("alpha beta gamma delta epsilon zeta eta theta"), &CandidateParams::default(), &[]).unwrap();
        assert!(c.is_empty());
    }
}
