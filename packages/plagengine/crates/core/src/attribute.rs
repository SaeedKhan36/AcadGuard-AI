//! Stage 5 — attribution. Every submission char counts toward at most one
//! source. Greedy set cover: repeatedly pick the source that explains the
//! most still-unclaimed countable chars, claim them, repeat. This yields the
//! Turnitin ranking (source #1 = largest contribution) and guarantees that
//! overlapping sources never double-count.

use crate::engine::Passage;
use crate::types::Span;
use std::collections::BTreeMap;

/// A (possibly clipped) piece of a passage attributed to one source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    /// Index into the passages slice given to [`attribute`].
    pub passage: usize,
    pub sub: Span,
    pub src: Span,
    pub matched_tokens: usize,
    pub chars: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribution {
    pub source_id: u32,
    pub pieces: Vec<Piece>,
    /// Countable chars attributed to this source.
    pub chars: usize,
}

#[derive(Clone, Debug)]
pub struct AttrParams {
    /// Drop pieces with fewer words (submission tokens).
    pub min_words: usize,
    /// Drop pieces with fewer matched tokens (floor for clipped leftovers).
    pub min_piece_tokens: usize,
    /// Stop once the best source explains less than this % of the denominator.
    pub min_percent: f64,
    /// Split a passage where more than this many unmatched words separate
    /// two matched tokens, so highlights never span long unmatched text.
    pub split_gap_words: usize,
    /// Split where the source side jumps forward by more than this many
    /// chars or moves backwards, so each match maps to one source region.
    pub split_src_gap_chars: usize,
}

impl Default for AttrParams {
    fn default() -> Self {
        AttrParams { min_words: 0, min_piece_tokens: 4, min_percent: 0.0, split_gap_words: 6, split_src_gap_chars: 60 }
    }
}

/// Inputs describing the submission.
pub struct Canvas<'a> {
    /// `prefix[i]` = countable chars in `[0, i)`.
    pub countable_prefix: &'a [u32],
    /// Chars that may not be attributed (excluded quotes / bibliography).
    pub blocked: &'a [bool],
    /// Words (tokens) starting inside a span.
    pub words: &'a dyn Fn(Span) -> usize,
    /// Denominator for `min_percent`.
    pub denominator: usize,
    /// Quotation spans (sorted). A passage is split where it crosses the
    /// boundary of a quote that touches its edge (the writer quoted part of
    /// the copied text); quotes wholly inside a passage — e.g. dialogue in a
    /// copied novel page — belong to the copied content and do not split.
    pub quotes: &'a [Span],
}

impl Canvas<'_> {
    fn countable(&self, s: Span) -> usize {
        (self.countable_prefix[s.end] - self.countable_prefix[s.start]) as usize
    }
}

fn prefix_of(mask: &[bool]) -> Vec<u32> {
    let mut p = Vec::with_capacity(mask.len() + 1);
    let mut acc = 0;
    p.push(0);
    for &b in mask {
        acc += b as u32;
        p.push(acc);
    }
    p
}

/// Pieces a source can still claim given `claimed`.
fn pieces_for(passages: &[Passage], idxs: &[usize], claimed: &[bool], canvas: &Canvas, p: &AttrParams) -> Vec<Piece> {
    let mut local: Vec<Span> = Vec::new(); // claimed by this source's earlier pieces
    let claimed_prefix = prefix_of(claimed);
    let is_free = |s: Span, local: &[Span]| {
        claimed_prefix[s.end] == claimed_prefix[s.start] && !local.iter().any(|l| l.overlaps(&s))
    };
    let mut out = Vec::new();
    for &pi in idxs {
        let pass = &passages[pi];
        let edge_quotes: Vec<Span> = canvas
            .quotes
            .iter()
            .filter(|q| q.overlaps(&pass.sub) && (q.start <= pass.sub.start || q.end >= pass.sub.end))
            .copied()
            .collect();
        let quoted = |pos: usize| edge_quotes.iter().any(|q| q.contains(pos));
        let mut run: Vec<usize> = Vec::new(); // indices into pass.pairs
        let flush = |run: &mut Vec<usize>, local: &mut Vec<Span>, out: &mut Vec<Piece>| {
            if run.is_empty() {
                return;
            }
            let first = pass.pairs[run[0]];
            let last = pass.pairs[*run.last().expect("non-empty")];
            let sub = Span::new(first.0, last.1);
            let src_start = run.iter().map(|&i| pass.pairs[i].2).min().expect("non-empty");
            let src_end = run.iter().map(|&i| pass.pairs[i].3).max().expect("non-empty");
            let piece = Piece {
                passage: pi,
                sub,
                src: Span::new(src_start, src_end),
                matched_tokens: run.len(),
                chars: canvas.countable(sub),
            };
            run.clear();
            if piece.matched_tokens >= p.min_piece_tokens && (canvas.words)(sub) >= p.min_words && piece.chars > 0 {
                local.push(sub);
                out.push(piece);
            }
        };
        for (i, pair) in pass.pairs.iter().enumerate() {
            let tok = pair.sub();
            if !is_free(tok, &local) {
                flush(&mut run, &mut local, &mut out);
                continue;
            }
            if let Some(&prev) = run.last() {
                let pp = pass.pairs[prev];
                let gap = Span::new(pp.1, tok.start.max(pp.1));
                let src_jump = pair.2 < pp.2 || pair.2.saturating_sub(pp.3) > p.split_src_gap_chars;
                let quote_change = !edge_quotes.is_empty() && quoted(pp.0) != quoted(tok.start);
                if src_jump || quote_change || (!gap.is_empty() && (!is_free(gap, &local) || (canvas.words)(gap) > p.split_gap_words)) {
                    flush(&mut run, &mut local, &mut out);
                }
            }
            run.push(i);
        }
        flush(&mut run, &mut local, &mut out);
    }
    out
}

/// Attribute passages to sources. Returns sources in rank order.
pub fn attribute(passages: &[Passage], canvas: &Canvas, p: &AttrParams) -> Vec<Attribution> {
    let mut by_source: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (i, pass) in passages.iter().enumerate() {
        by_source.entry(pass.source_id).or_default().push(i);
    }
    for idxs in by_source.values_mut() {
        idxs.sort_by_key(|&i| (std::cmp::Reverse(passages[i].matched_tokens), passages[i].sub.start));
    }
    let mut claimed: Vec<bool> = canvas.blocked.to_vec();
    let mut ranked = Vec::new();
    while !by_source.is_empty() {
        let mut best: Option<(usize, u32, Vec<Piece>)> = None;
        for (&sid, idxs) in &by_source {
            let pieces = pieces_for(passages, idxs, &claimed, canvas, p);
            let gain: usize = pieces.iter().map(|x| x.chars).sum();
            if best.as_ref().is_none_or(|(g, _, _)| gain > *g) {
                best = Some((gain, sid, pieces));
            }
        }
        let Some((gain, sid, mut pieces)) = best else { break };
        let pct = if canvas.denominator == 0 { 0.0 } else { gain as f64 * 100.0 / canvas.denominator as f64 };
        if gain == 0 || pct < p.min_percent {
            break;
        }
        for piece in &pieces {
            for c in claimed.iter_mut().take(piece.sub.end).skip(piece.sub.start) {
                *c = true;
            }
        }
        pieces.sort_by_key(|x| x.sub.start);
        by_source.remove(&sid);
        ranked.push(Attribution { source_id: sid, pieces, chars: gain });
    }
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::align::TokenPair;

    /// Passage over "words" of 4 chars + 1 space: token t occupies [5t, 5t+4).
    fn passage(source_id: u32, from_tok: usize, to_tok: usize, src_offset: usize) -> Passage {
        let pairs: Vec<TokenPair> = (from_tok..to_tok)
            .map(|t| TokenPair(5 * t, 5 * t + 4, 5 * t + src_offset, 5 * t + 4 + src_offset))
            .collect();
        Passage {
            source_id,
            sub: Span::new(5 * from_tok, 5 * to_tok - 1),
            src: Span::new(5 * from_tok + src_offset, 5 * to_tok - 1 + src_offset),
            matched_tokens: to_tok - from_tok,
            pairs,
        }
    }

    fn run(passages: &[Passage], blocked: Option<Span>, p: &AttrParams) -> Vec<Attribution> {
        let n = 500;
        let chars: Vec<char> = (0..n).map(|i| if i % 5 == 4 { ' ' } else { 'x' }).collect();
        let mut prefix = vec![0u32];
        for c in &chars {
            prefix.push(prefix.last().unwrap() + (*c != ' ') as u32);
        }
        let mut mask = vec![false; n];
        if let Some(b) = blocked {
            mask[b.start..b.end].iter_mut().for_each(|m| *m = true);
        }
        let words = |s: Span| (s.start..s.end).filter(|i| i % 5 == 0).count();
        let canvas = Canvas { countable_prefix: &prefix, blocked: &mask, words: &words, denominator: 400, quotes: &[] };
        attribute(passages, &canvas, p)
    }

    #[test]
    fn overlap_counts_once_and_ranks_by_contribution() {
        // Source 1: tokens 0..30, source 2: tokens 20..40 (overlap 20..30).
        let ps = vec![passage(1, 0, 30, 0), passage(2, 20, 40, 100)];
        let a = run(&ps, None, &AttrParams::default());
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].source_id, 1);
        assert_eq!(a[0].chars, 30 * 4);
        assert_eq!(a[1].source_id, 2);
        assert_eq!(a[1].chars, 10 * 4, "only the non-overlapping part");
        assert_eq!(a[1].pieces[0].sub, Span::new(150, 199));
        // Clipped piece maps back to the exact source range via token pairs.
        assert_eq!(a[1].pieces[0].src, Span::new(250, 299));
    }

    #[test]
    fn fully_covered_source_drops_out() {
        let ps = vec![passage(1, 0, 30, 0), passage(2, 5, 25, 0)];
        let a = run(&ps, None, &AttrParams::default());
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn min_words_and_min_percent_and_blocked() {
        let ps = vec![passage(1, 0, 30, 0), passage(2, 40, 46, 0), passage(3, 50, 70, 0)];
        let a = run(&ps, None, &AttrParams { min_words: 8, ..Default::default() });
        assert_eq!(a.iter().map(|x| x.source_id).collect::<Vec<_>>(), vec![1, 3]);
        // 20 tokens * 4 chars = 80/400 = 20% -> excluded at 25%.
        let a = run(&ps, None, &AttrParams { min_percent: 25.0, ..Default::default() });
        assert_eq!(a.iter().map(|x| x.source_id).collect::<Vec<_>>(), vec![1]);
        // Blocking the middle of source 1 splits it into two pieces.
        let a = run(&ps, Some(Span::new(50, 100)), &AttrParams::default());
        assert_eq!(a[0].source_id, 1);
        assert_eq!(a[0].pieces.len(), 2);
        assert_eq!(a[0].chars, 20 * 4);
    }

    #[test]
    fn long_internal_gaps_split_pieces() {
        // One passage whose pairs skip tokens 10..20 (unmatched words).
        let mut p = passage(1, 0, 30, 0);
        p.pairs.retain(|tp| !(50..100).contains(&tp.0));
        let a = run(&[p], None, &AttrParams::default());
        assert_eq!(a[0].pieces.len(), 2);
        assert_eq!(a[0].chars, 20 * 4);
    }

    #[test]
    fn source_jumps_split_pieces() {
        // Submission tokens 0..20 contiguous; source jumps by 500 chars at token 10.
        let mut p = passage(1, 0, 20, 0);
        for tp in p.pairs.iter_mut().skip(10) {
            tp.2 += 500;
            tp.3 += 500;
        }
        let a = run(&[p], None, &AttrParams::default());
        assert_eq!(a[0].pieces.len(), 2);
        assert_eq!(a[0].pieces[1].src, Span::new(550, 599));
    }

    #[test]
    fn same_source_overlapping_passages_do_not_double_count() {
        let ps = vec![passage(1, 0, 20, 0), passage(1, 10, 30, 200)];
        let a = run(&ps, None, &AttrParams::default());
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].chars, 30 * 4);
    }
}
