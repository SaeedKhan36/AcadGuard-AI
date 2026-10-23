//! Evaluation: PAN character-level precision / recall / granularity /
//! plagdet (Potthast et al., "An Evaluation Framework for Plagiarism
//! Detection", COLING 2010), the PAN-PC-11 runner and the adversarial set.

pub mod adversarial;
pub mod pan;

use crate::types::{merge_spans, Span};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Ground-truth plagiarism case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Case {
    pub susp: String,
    pub src: String,
    pub susp_span: Span,
    pub src_span: Span,
    /// e.g. "artificial-low", "translation-automatic", "simulated".
    pub kind: String,
}

/// A detection reported by the engine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Detection {
    pub susp: String,
    pub src: String,
    pub susp_span: Span,
    pub src_span: Span,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Metrics {
    pub cases: usize,
    pub detections: usize,
    pub detected_cases: usize,
    pub precision: f64,
    pub recall: f64,
    pub granularity: f64,
    pub plagdet: f64,
}

fn detects(c: &Case, d: &Detection) -> bool {
    c.susp == d.susp && c.src == d.src && c.susp_span.overlaps(&d.susp_span) && c.src_span.overlaps(&d.src_span)
}

fn covered(target: Span, others: impl Iterator<Item = Span>) -> usize {
    merge_spans(others.filter_map(|o| o.intersect(&target)).collect()).iter().map(|s| s.len()).sum()
}

/// Compute PAN macro-averaged metrics.
pub fn evaluate(cases: &[Case], dets: &[Detection]) -> Metrics {
    // Index detections by document pair.
    let mut by_pair: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
    for (i, d) in dets.iter().enumerate() {
        by_pair.entry((d.susp.as_str(), d.src.as_str())).or_default().push(i);
    }
    let mut case_by_pair: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
    for (i, c) in cases.iter().enumerate() {
        case_by_pair.entry((c.susp.as_str(), c.src.as_str())).or_default().push(i);
    }

    let mut rec_sum = 0.0;
    let mut detected = 0usize;
    let mut gran_sum = 0usize;
    for c in cases {
        let ds: Vec<&Detection> = by_pair
            .get(&(c.susp.as_str(), c.src.as_str()))
            .map(|v| v.iter().map(|&i| &dets[i]).filter(|d| detects(c, d)).collect())
            .unwrap_or_default();
        if ds.is_empty() {
            continue;
        }
        detected += 1;
        gran_sum += ds.len();
        let num = covered(c.susp_span, ds.iter().map(|d| d.susp_span)) + covered(c.src_span, ds.iter().map(|d| d.src_span));
        rec_sum += num as f64 / (c.susp_span.len() + c.src_span.len()).max(1) as f64;
    }
    let mut prec_sum = 0.0;
    for d in dets {
        let cs: Vec<&Case> = case_by_pair
            .get(&(d.susp.as_str(), d.src.as_str()))
            .map(|v| v.iter().map(|&i| &cases[i]).filter(|c| detects(c, d)).collect())
            .unwrap_or_default();
        if cs.is_empty() {
            continue;
        }
        let num = covered(d.susp_span, cs.iter().map(|c| c.susp_span)) + covered(d.src_span, cs.iter().map(|c| c.src_span));
        prec_sum += num as f64 / (d.susp_span.len() + d.src_span.len()).max(1) as f64;
    }
    let recall = if cases.is_empty() { 0.0 } else { rec_sum / cases.len() as f64 };
    let precision = if dets.is_empty() { 0.0 } else { prec_sum / dets.len() as f64 };
    let granularity = if detected == 0 { 1.0 } else { gran_sum as f64 / detected as f64 };
    let f1 = if precision + recall == 0.0 { 0.0 } else { 2.0 * precision * recall / (precision + recall) };
    Metrics {
        cases: cases.len(),
        detections: dets.len(),
        detected_cases: detected,
        precision,
        recall,
        granularity,
        plagdet: f1 / (1.0 + granularity).log2(),
    }
}

/// Metrics per case kind. Precision per kind uses only detections that
/// detect at least one case of that kind (PAN convention for per-type
/// breakdowns), so it is mainly the recall/granularity columns that matter.
pub fn evaluate_by_kind(cases: &[Case], dets: &[Detection]) -> BTreeMap<String, Metrics> {
    let mut kinds: BTreeMap<String, Vec<Case>> = BTreeMap::new();
    for c in cases {
        kinds.entry(c.kind.clone()).or_default().push(c.clone());
    }
    kinds
        .into_iter()
        .map(|(k, cs)| {
            let ds: Vec<Detection> = dets.iter().filter(|d| cs.iter().any(|c| detects(c, d))).cloned().collect();
            (k, evaluate(&cs, &ds))
        })
        .collect()
}

/// Deterministic xorshift RNG (no external deps; reproducible eval sets).
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    pub fn chance(&mut self, p: f64) -> bool {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64 * 1.0 < p
    }
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(s: (usize, usize), r: (usize, usize)) -> Case {
        Case { susp: "a".into(), src: "b".into(), susp_span: Span::new(s.0, s.1), src_span: Span::new(r.0, r.1), kind: "x".into() }
    }
    fn det(s: (usize, usize), r: (usize, usize)) -> Detection {
        Detection { susp: "a".into(), src: "b".into(), susp_span: Span::new(s.0, s.1), src_span: Span::new(r.0, r.1) }
    }

    #[test]
    fn perfect_detection() {
        let m = evaluate(&[case((0, 100), (50, 150))], &[det((0, 100), (50, 150))]);
        assert_eq!((m.precision, m.recall, m.granularity, m.plagdet), (1.0, 1.0, 1.0, 1.0));
    }

    #[test]
    fn half_detection_and_granularity() {
        // Two detections each covering half of the case: recall 1, gran 2.
        let c = [case((0, 100), (0, 100))];
        let d = [det((0, 50), (0, 50)), det((50, 100), (50, 100))];
        let m = evaluate(&c, &d);
        assert_eq!(m.recall, 1.0);
        assert_eq!(m.precision, 1.0);
        assert_eq!(m.granularity, 2.0);
        assert!((m.plagdet - 1.0 / 3f64.log2()).abs() < 1e-9);
        // A detection twice the size of the case: precision 0.5.
        let m = evaluate(&c, &[det((0, 200), (0, 200))]);
        assert!((m.precision - 0.5).abs() < 1e-9 && m.recall == 1.0);
        // Wrong source pair detects nothing.
        let mut wrong = det((0, 100), (0, 100));
        wrong.src = "z".into();
        let m = evaluate(&c, &[wrong]);
        assert_eq!((m.precision, m.recall, m.plagdet), (0.0, 0.0, 0.0));
    }

    #[test]
    fn rng_is_deterministic() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        assert_eq!((0..10).map(|_| a.below(100)).collect::<Vec<_>>(), (0..10).map(|_| b.below(100)).collect::<Vec<_>>());
    }
}
