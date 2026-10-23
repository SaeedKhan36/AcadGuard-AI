//! Stage 4 — seed-and-extend alignment between a submission and one source.
//!
//! 1. Seeds: every k-gram (small k, default 3) shared by both token streams.
//! 2. Extend: seeds on the same diagonal are joined and extended token by
//!    token into maximal exact runs ("fragments").
//! 3. Chain: fragments are greedily chained into passages when the gap to the
//!    passage is at most `max_gap` tokens on both sides (tolerates inserted,
//!    deleted or substituted words), then passages separated by at most
//!    `merge_gap` tokens are merged while density stays above `min_density`.
//! 4. Filter: keep passages with ≥ `min_tokens` matched tokens.
//!
//! Output ranges are exact char ranges on both sides, plus the aligned token
//! pairs so a passage can later be clipped without losing its source mapping.

use crate::normalize::NormDoc;
use crate::types::Span;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

/// Hasher for keys that already are well-mixed 64-bit hashes.
#[derive(Default)]
pub struct IdHasher(u64);
impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = self.0.rotate_left(8) ^ b as u64;
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }
}
pub type IdMap<V> = HashMap<u64, V, BuildHasherDefault<IdHasher>>;

#[derive(Clone, Debug, PartialEq)]
pub struct AlignParams {
    /// Seed k-gram length in tokens.
    pub k: usize,
    /// Max gap (tokens, either side) when chaining fragments.
    pub max_gap: usize,
    /// Max gap (tokens, either side) when merging passages.
    pub merge_gap: usize,
    /// Minimum matched tokens for a passage to be reported.
    pub min_tokens: usize,
    /// Minimum matched tokens / passage length (submission side).
    pub min_density: f64,
    /// Seeds whose k-gram occurs more often than this on either side are ignored.
    pub max_occ: usize,
    /// Second, coarse merge tier for heavily edited passages: max gap in
    /// tokens (0 disables).
    pub coarse_gap: usize,
    /// Minimum density of a coarse-merged passage.
    pub coarse_min_density: f64,
    /// Diagonal drift always tolerated by the coarse tier (tokens); larger
    /// gaps tolerate up to half the gap.
    pub coarse_drift: usize,
    /// Final grouping of reported passages that lie within this many tokens
    /// of each other on both sides, in either source order (obfuscation often
    /// reorders sentences). Report highlights still split at internal gaps.
    pub final_gap: usize,
    /// A reported passage must contain a fine-tier (lightly edited) run of
    /// at least this many matched tokens; stops the coarse tier from
    /// stitching scraps of common phrases together.
    pub min_core: usize,
}

impl Default for AlignParams {
    fn default() -> Self {
        AlignParams {
            k: 3,
            max_gap: 5,
            merge_gap: 10,
            min_tokens: 8,
            min_density: 0.5,
            max_occ: 20,
            coarse_gap: 30,
            coarse_min_density: 0.2,
            coarse_drift: 10,
            final_gap: 60,
            min_core: 6,
        }
    }
}

/// Tolerated overlap (tokens) between consecutive fragments of a chain.
const OVERLAP: isize = 2;

/// Exact run of `len` equal tokens at `q` (submission) and `s` (source).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fragment {
    pub q: usize,
    pub s: usize,
    pub len: usize,
}

/// A passage in token coordinates (end-exclusive).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokPassage {
    pub q_start: usize,
    pub q_end: usize,
    pub s_start: usize,
    pub s_end: usize,
    pub frags: Vec<Fragment>,
    pub matched: usize,
    /// Matched tokens of the largest fine-tier passage inside.
    pub core: usize,
}

impl TokPassage {
    fn from_frag(f: Fragment) -> Self {
        TokPassage { q_start: f.q, q_end: f.q + f.len, s_start: f.s, s_end: f.s + f.len, frags: vec![f], matched: f.len, core: f.len }
    }
    fn density(&self) -> f64 {
        self.matched as f64 / (self.q_end - self.q_start).max(1) as f64
    }
    fn recompute(&mut self) {
        self.frags.sort();
        let coverage = |mut iv: Vec<(usize, usize)>| {
            iv.sort_unstable();
            let (mut covered, mut end) = (0usize, 0usize);
            for (s, e) in iv {
                let s = s.max(end);
                if e > s {
                    covered += e - s;
                }
                end = end.max(e);
            }
            covered
        };
        let q_cov = coverage(self.frags.iter().map(|f| (f.q, f.q + f.len)).collect());
        let s_cov = coverage(self.frags.iter().map(|f| (f.s, f.s + f.len)).collect());
        self.matched = q_cov.min(s_cov);
        self.q_start = self.frags.iter().map(|f| f.q).min().unwrap_or(0);
        self.q_end = self.frags.iter().map(|f| f.q + f.len).max().unwrap_or(0);
        self.s_start = self.frags.iter().map(|f| f.s).min().unwrap_or(0);
        self.s_end = self.frags.iter().map(|f| f.s + f.len).max().unwrap_or(0);
    }
}

/// Aligned token pair as char ranges: `(sub_start, sub_end, src_start, src_end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TokenPair(pub usize, pub usize, pub usize, pub usize);

impl TokenPair {
    pub fn sub(&self) -> Span {
        Span::new(self.0, self.1)
    }
    pub fn src(&self) -> Span {
        Span::new(self.2, self.3)
    }
}

/// A passage in char coordinates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharAlignment {
    pub sub: Span,
    pub src: Span,
    pub matched_tokens: usize,
    pub pairs: Vec<TokenPair>,
}

fn distance(a_end: usize, b_start: usize) -> isize {
    b_start as isize - a_end as isize
}

/// Query-side seed index, built once per check and shared by every
/// candidate alignment.
pub struct QueryIndex {
    k: usize,
    tokens: Vec<u64>,
    map: IdMap<Vec<u32>>,
}

impl QueryIndex {
    pub fn new(q: &[u64], k: usize, max_occ: usize) -> Self {
        let mut map: IdMap<Vec<u32>> = IdMap::default();
        if k > 0 && q.len() >= k {
            for (i, h) in crate::fingerprint::kgram_hashes(q, k).into_iter().enumerate() {
                map.entry(h).or_default().push(i as u32);
            }
        }
        map.retain(|_, v| v.len() <= max_occ);
        QueryIndex { k, tokens: q.to_vec(), map }
    }

    /// Exact-match fragments between the query and source tokens `s`.
    pub fn fragments(&self, s: &[u64], max_occ: usize) -> Vec<Fragment> {
        let (q, k) = (&self.tokens, self.k);
        if q.len() < k || s.len() < k || k == 0 || self.map.is_empty() {
            return Vec::new();
        }
        // Source k-grams hashed on the fly; only those present in the query matter.
        let mut hits: Vec<(u64, u32)> = Vec::new();
        for (j, w) in s.windows(k).enumerate() {
            let h = crate::fingerprint::hash_window(w);
            if self.map.contains_key(&h) {
                hits.push((h, j as u32));
            }
        }
        let mut s_occ: IdMap<u32> = IdMap::default();
        for (h, _) in &hits {
            *s_occ.entry(*h).or_default() += 1;
        }
        // Seeds as (diagonal, q) so runs on one diagonal are consecutive.
        let mut seeds: Vec<(isize, usize)> = Vec::new();
        for (h, j) in hits {
            if s_occ[&h] as usize > max_occ {
                continue;
            }
            for &i in &self.map[&h] {
                seeds.push((j as isize - i as isize, i as usize));
            }
        }
        seeds.sort_unstable();

        let mut frags: Vec<Fragment> = Vec::new();
        let mut seen: HashSet<(usize, usize)> = HashSet::new();
        let mut idx = 0;
        while idx < seeds.len() {
            let (d, i0) = seeds[idx];
            let mut end = i0 + k;
            idx += 1;
            while idx < seeds.len() && seeds[idx].0 == d && seeds[idx].1 <= end {
                end = end.max(seeds[idx].1 + k);
                idx += 1;
            }
            // Extend by direct comparison (picks up k-grams dropped by max_occ).
            let mut qs = i0;
            let mut ss = (i0 as isize + d) as usize;
            while qs > 0 && ss > 0 && q[qs - 1] == s[ss - 1] {
                qs -= 1;
                ss -= 1;
            }
            let mut qe = end;
            let mut se = (end as isize + d) as usize;
            while qe < q.len() && se < s.len() && q[qe] == s[se] {
                qe += 1;
                se += 1;
            }
            if seen.insert((qs, ss)) {
                frags.push(Fragment { q: qs, s: ss, len: qe - qs });
            }
        }
        frags.sort_unstable();
        frags
    }
}

/// Find exact-match fragments between two token streams.
pub fn fragments(q: &[u64], s: &[u64], k: usize, max_occ: usize) -> Vec<Fragment> {
    QueryIndex::new(q, k, max_occ).fragments(s, max_occ)
}

/// Greedy banded chaining of units (fragments or passages) sorted by
/// submission position. A unit joins the cluster that ends just before it
/// on both sides (gap ≤ `gap`, overlap ≤ [`OVERLAP`]) when the diagonal
/// drift `|dq - ds|` is within `max_drift(dq, ds)` and the cluster's density
/// stays ≥ `min_density`. Active clusters are indexed by source end so each
/// unit only inspects clusters ending near its source start.
fn chain_units(
    mut units: Vec<TokPassage>,
    gap: usize,
    max_drift: impl Fn(isize, isize) -> isize,
    min_density: f64,
) -> Vec<TokPassage> {
    use std::collections::BTreeSet;
    units.sort_by_key(|u| (u.q_start, u.s_start));
    let mut clusters: Vec<TokPassage> = Vec::with_capacity(units.len());
    let mut active: BTreeSet<(usize, usize)> = BTreeSet::new(); // (s_end, cluster index)
    let igap = gap as isize;
    for u in units {
        let lo = u.s_start.saturating_sub(gap);
        let hi = u.s_start + OVERLAP as usize;
        let mut best: Option<(isize, usize)> = None;
        let mut expired = Vec::new();
        for &(s_end, ci) in active.range((lo, 0)..=(hi, usize::MAX)) {
            let c = &clusters[ci];
            if c.q_end + gap < u.q_start {
                expired.push((s_end, ci));
                continue;
            }
            let dq = distance(c.q_end, u.q_start);
            let ds = distance(c.s_end, u.s_start);
            if dq < -OVERLAP || ds < -OVERLAP || dq > igap || ds > igap {
                continue;
            }
            if u.q_end <= c.q_end || u.s_end <= c.s_end || (dq - ds).abs() > max_drift(dq, ds) {
                continue;
            }
            let cost = dq.max(0) + ds.max(0) + (dq - ds).abs();
            if best.is_none_or(|(b, _)| cost < b) {
                best = Some((cost, ci));
            }
        }
        for e in expired {
            active.remove(&e);
        }
        let joined = best.and_then(|(_, ci)| {
            let c = &clusters[ci];
            let mut trial = c.clone();
            trial.core = trial.core.max(u.core);
            if u.q_start >= c.q_end && u.s_start >= c.s_end {
                trial.q_end = u.q_end;
                trial.s_end = u.s_end;
                trial.matched += u.matched;
                trial.frags.extend(u.frags.iter().copied());
            } else {
                trial.frags.extend(u.frags.iter().copied());
                trial.recompute();
            }
            (trial.density() >= min_density).then_some((ci, trial))
        });
        match joined {
            Some((ci, trial)) => {
                active.remove(&(clusters[ci].s_end, ci));
                active.insert((trial.s_end, ci));
                clusters[ci] = trial;
            }
            None => {
                active.insert((u.s_end, clusters.len()));
                clusters.push(u);
            }
        }
    }
    clusters
}

/// Chain fragments into passages (token coordinates): a fine tier for exact
/// and lightly edited copies, then an optional coarse tier for heavily
/// edited passages whose surviving fragments lie along a diagonal band.
pub fn chain(frags: &[Fragment], p: &AlignParams) -> Vec<TokPassage> {
    let units: Vec<TokPassage> = frags.iter().map(|&f| TokPassage::from_frag(f)).collect();
    let fine = chain_units(units, p.max_gap, |_, _| isize::MAX, 0.0);
    let mut fine = merge_passages(fine, p.merge_gap, p.min_density);
    for f in &mut fine {
        f.core = f.matched;
    }
    if p.coarse_gap == 0 {
        return fine;
    }
    let drift = |dq: isize, ds: isize| (p.coarse_drift as isize).max(dq.max(ds) / 2);
    let coarse = chain_units(fine, p.coarse_gap, drift, p.coarse_min_density);
    chain_units(coarse, p.coarse_gap, drift, p.coarse_min_density)
}

fn merge_passages(mut passages: Vec<TokPassage>, gap: usize, min_density: f64) -> Vec<TokPassage> {
    let gap = gap as isize;
    loop {
        passages.sort_by_key(|c| (c.q_start, c.s_start));
        let mut merged: Vec<TokPassage> = Vec::with_capacity(passages.len());
        let mut changed = false;
        'outer: for c in passages {
            for m in merged.iter_mut().rev() {
                if m.q_end as isize + gap < c.q_start as isize {
                    break;
                }
                let dq = distance(m.q_end, c.q_start);
                let ds = distance(m.s_end, c.s_start);
                if dq > gap || ds > gap || ds < -OVERLAP || dq < -OVERLAP || c.s_end <= m.s_end || c.q_end <= m.q_end {
                    continue;
                }
                let mut trial = m.clone();
                trial.frags.extend(c.frags.iter().copied());
                trial.recompute();
                trial.core = trial.core.max(c.core);
                if trial.density() >= min_density {
                    *m = trial;
                    changed = true;
                    continue 'outer;
                }
            }
            merged.push(c);
        }
        passages = merged;
        if !changed {
            return passages;
        }
    }
}

/// Keep reportable passages: enough tokens, dense enough, and not mostly
/// covered (submission side) by a better passage.
fn select(mut passages: Vec<TokPassage>, p: &AlignParams) -> Vec<TokPassage> {
    let density = if p.coarse_gap > 0 { p.min_density.min(p.coarse_min_density) } else { p.min_density };
    passages.retain(|c| c.matched >= p.min_tokens && c.core >= p.min_core.min(p.min_tokens) && c.density() >= density);
    passages.sort_by(|a, b| b.matched.cmp(&a.matched).then(a.q_start.cmp(&b.q_start)));
    let mut kept: Vec<TokPassage> = Vec::new();
    for c in passages {
        let len = c.q_end - c.q_start;
        let overlap: usize = kept
            .iter()
            .map(|k| c.q_end.min(k.q_end).saturating_sub(c.q_start.max(k.q_start)))
            .sum();
        if overlap * 2 < len {
            kept.push(c);
        }
    }
    kept.sort_by_key(|c| (c.q_start, c.s_start));
    kept
}

/// Group selected passages that are close on both sides (any source order).
fn merge_final(mut ps: Vec<TokPassage>, gap: usize) -> Vec<TokPassage> {
    if gap == 0 {
        return ps;
    }
    loop {
        ps.sort_by_key(|c| (c.q_start, c.s_start));
        let mut out: Vec<TokPassage> = Vec::with_capacity(ps.len());
        let mut changed = false;
        for c in ps {
            let near = out.iter_mut().rev().find(|m| {
                let s_dist = c.s_start.saturating_sub(m.s_end).max(m.s_start.saturating_sub(c.s_end));
                c.q_start <= m.q_end + gap && s_dist <= gap
            });
            match near {
                Some(m) => {
                    m.frags.extend(c.frags);
                    m.recompute();
                    m.core = m.core.max(c.core);
                    changed = true;
                }
                None => out.push(c),
            }
        }
        ps = out;
        if !changed {
            return ps;
        }
    }
}

/// Align token streams; returns passages in token coordinates.
pub fn align_tokens(q: &[u64], s: &[u64], p: &AlignParams) -> Vec<TokPassage> {
    let frags = fragments(q, s, p.k, p.max_occ);
    merge_final(select(chain(&frags, p), p), p.final_gap)
}

/// Align two normalized documents; returns passages in char coordinates.
pub fn align(q: &NormDoc, s: &NormDoc, p: &AlignParams) -> Vec<CharAlignment> {
    align_indexed(&QueryIndex::new(&q.token_hashes(), p.k, p.max_occ), q, s, p)
}

/// [`align`] with a prebuilt query index (one per check).
pub fn align_indexed(qi: &QueryIndex, q: &NormDoc, s: &NormDoc, p: &AlignParams) -> Vec<CharAlignment> {
    let sh = s.token_hashes();
    let frags = qi.fragments(&sh, p.max_occ);
    merge_final(select(chain(&frags, p), p), p.final_gap)
        .into_iter()
        .map(|tp| {
            let mut pairs = Vec::with_capacity(tp.matched);
            let mut next_q = 0usize;
            for f in &tp.frags {
                for t in 0..f.len {
                    let qi = f.q + t;
                    if qi < next_q {
                        continue;
                    }
                    let (qt, st) = (q.tokens[qi], s.tokens[f.s + t]);
                    pairs.push(TokenPair(qt.start, qt.end, st.start, st.end));
                    next_q = qi + 1;
                }
            }
            CharAlignment {
                sub: Span::new(q.tokens[tp.q_start].start, q.tokens[tp.q_end - 1].end),
                src: Span::new(s.tokens[tp.s_start].start, s.tokens[tp.s_end - 1].end),
                matched_tokens: tp.matched,
                pairs,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::normalize;
    use crate::types::slice_chars;

    const SRC: &str = "Climate change refers to long-term shifts in temperatures and weather patterns. \
        Such shifts can be natural, due to changes in the sun's activity or large volcanic eruptions. \
        But since the 1800s, human activities have been the main driver of climate change, primarily \
        due to the burning of fossil fuels like coal, oil and gas. Burning fossil fuels generates \
        greenhouse gas emissions that act like a blanket wrapped around the Earth, trapping the sun's \
        heat and raising temperatures.";

    fn run(sub: &str, src: &str) -> (NormDoc, NormDoc, Vec<CharAlignment>) {
        run_with(sub, src, &AlignParams::default())
    }

    fn run_with(sub: &str, src: &str, p: &AlignParams) -> (NormDoc, NormDoc, Vec<CharAlignment>) {
        let q = normalize(sub, &[]);
        let s = normalize(src, &[]);
        let a = align(&q, &s, p);
        (q, s, a)
    }

    #[test]
    fn exact_copy_has_exact_ranges() {
        let copied = "human activities have been the main driver of climate change, primarily due to the burning of fossil fuels";
        let sub = format!("My own introduction sentence here. {copied}. And my own conclusion follows.");
        let (q, s, a) = run(&sub, SRC);
        assert_eq!(a.len(), 1);
        assert_eq!(slice_chars(&q.chars, a[0].sub), copied);
        assert_eq!(slice_chars(&s.chars, a[0].src), copied);
        assert_eq!(a[0].matched_tokens, 18);
        assert_eq!(a[0].pairs.len(), 18);
    }

    #[test]
    fn light_edits_merge_into_one_passage() {
        // substitutions ("main"→"primary", "like"→"such as"), a deletion and an insertion
        let sub = "But since the 1800s, human activities have been the primary driver of climate change, \
            mostly due to the burning of fossil fuels such as coal, oil and natural gas. Burning fossil fuels \
            generates greenhouse gas emissions that act like a blanket wrapped around the Earth.";
        let (q, _, a) = run(sub, SRC);
        assert_eq!(a.len(), 1, "{a:?}");
        let text = slice_chars(&q.chars, a[0].sub);
        assert!(text.starts_with("But since the 1800s"));
        assert!(text.ends_with("around the Earth"));
    }

    #[test]
    fn reordered_sentences_give_two_passages() {
        let sub = "Burning fossil fuels generates greenhouse gas emissions that act like a blanket wrapped around the Earth. \
            Some words of my own go right here to separate the two parts clearly enough for the test. \
            Climate change refers to long-term shifts in temperatures and weather patterns.";
        let (_, s, a) = run_with(sub, SRC, &AlignParams { final_gap: 0, ..Default::default() });
        assert_eq!(a.len(), 2);
        assert!(a[0].src.start > a[1].src.start);
        assert!(slice_chars(&s.chars, a[1].src).starts_with("Climate change refers"));
        // With final grouping they form one passage whose pairs keep both mappings.
        let (_, _, g) = run(sub, SRC);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].pairs.len(), a[0].pairs.len() + a[1].pairs.len());
    }

    #[test]
    fn unrelated_and_short_matches_are_ignored() {
        let (_, _, a) = run("The burning of fossil fuels is bad. Cats are wonderful animals that sleep a lot.", SRC);
        assert!(a.is_empty(), "{a:?}");
    }

    #[test]
    fn homoglyph_and_zero_width_copy_still_aligns() {
        let sub = "Вut sinсe the 1800s, humаn activ\u{200B}ities have been the main dri\u{200B}ver of climate change, primarily due to the burning";
        let (q, _, a) = run(sub, SRC);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].sub, Span::new(0, q.chars.len()));
    }

    #[test]
    fn repeated_phrase_does_not_chain_with_itself() {
        let p = AlignParams { min_tokens: 6, ..Default::default() };
        // q repeats "a b c" three times; s has it once.
        let q: Vec<u64> = vec![1, 2, 3, 9, 1, 2, 3, 8, 1, 2, 3, 7];
        let s: Vec<u64> = vec![1, 2, 3, 4];
        assert!(align_tokens(&q, &s, &p).is_empty());
    }

    #[test]
    fn coarse_tier_joins_heavily_edited_passage() {
        // Shared runs of 5 tokens separated by 12 unrelated tokens on both sides.
        let mut q = Vec::new();
        let mut s = Vec::new();
        for block in 0..6u64 {
            for t in 0..5 {
                q.push(block * 100 + t);
                s.push(block * 100 + t);
            }
            for t in 0..12 {
                q.push(10_000 + block * 100 + t);
                s.push(20_000 + block * 100 + t);
            }
        }
        let fine_only = AlignParams { coarse_gap: 0, final_gap: 0, min_tokens: 5, ..Default::default() };
        assert_eq!(align_tokens(&q, &s, &fine_only).len(), 6);
        let coarse = AlignParams { min_tokens: 5, min_core: 5, ..Default::default() };
        let out = align_tokens(&q, &s, &coarse);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].matched, 30);
    }

    #[test]
    fn final_grouping_handles_reordered_blocks() {
        // Two 10-token blocks copied in swapped order, 5 own tokens between.
        let a: Vec<u64> = (0..10).collect();
        let b: Vec<u64> = (100..110).collect();
        let s: Vec<u64> = a.iter().chain(b.iter()).copied().collect();
        let q: Vec<u64> = b.iter().chain([900, 901, 902, 903, 904].iter()).chain(a.iter()).copied().collect();
        let p = AlignParams { min_tokens: 8, ..Default::default() };
        let out = align_tokens(&q, &s, &p);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].matched, 20);
        let separate = AlignParams { final_gap: 0, ..p };
        assert_eq!(align_tokens(&q, &s, &separate).len(), 2);
    }

    #[test]
    fn token_level_chain() {
        let p = AlignParams { min_tokens: 1, ..Default::default() };
        let q: Vec<u64> = (0..30).collect();
        let mut s: Vec<u64> = (0..30).collect();
        s[10] = 999; // one substitution
        s.insert(20, 998); // one insertion
        let out = align_tokens(&q, &s, &p);
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].q_start, out[0].q_end, out[0].s_start, out[0].s_end), (0, 30, 0, 31));
        assert_eq!(out[0].matched, 29);
    }
}
