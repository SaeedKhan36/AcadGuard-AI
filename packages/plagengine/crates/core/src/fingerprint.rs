//! Stage 2 — k-gram hashing and winnowing (Schleimer, Wilkerson & Aiken 2003).
//!
//! Winnowing with window `w` over k-gram hashes guarantees that any shared run
//! of at least `w + k - 1` tokens yields at least one common fingerprint.

/// Default k-gram length (tokens) for the index.
pub const DEFAULT_K: usize = 4;
/// Default winnowing window.
pub const DEFAULT_W: usize = 3;

#[inline]
fn mix(mut h: u64) -> u64 {
    // splitmix64 finalizer
    h ^= h >> 30;
    h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^ (h >> 31)
}

/// Order-sensitive hash of a window of token hashes.
#[inline]
pub fn hash_window(window: &[u64]) -> u64 {
    let mut h = 0x9e37_79b9_7f4a_7c15u64 ^ window.len() as u64;
    for &t in window {
        h = mix(h.rotate_left(23) ^ t);
    }
    h
}

/// Hash every k-gram. `out[i]` covers tokens `[i, i + k)`. Empty when fewer
/// than `k` tokens.
pub fn kgram_hashes(tokens: &[u64], k: usize) -> Vec<u64> {
    assert!(k > 0);
    if tokens.len() < k {
        return Vec::new();
    }
    tokens.windows(k).map(hash_window).collect()
}

/// A selected fingerprint: k-gram hash and its k-gram index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    pub hash: u64,
    pub pos: u32,
}

/// Winnow k-gram hashes with window `w`: in every window of `w` consecutive
/// hashes select the minimum (rightmost on ties); emit each selection once.
pub fn winnow(hashes: &[u64], w: usize) -> Vec<Fingerprint> {
    assert!(w > 0);
    let n = hashes.len();
    let mut out = Vec::with_capacity(n * 2 / (w + 1) + 1);
    if n == 0 {
        return out;
    }
    if n <= w {
        let (pos, &hash) = hashes.iter().enumerate().rev().min_by_key(|(_, h)| **h).expect("non-empty");
        out.push(Fingerprint { hash, pos: pos as u32 });
        return out;
    }
    // Monotonic deque of indices with increasing hash values.
    let mut dq: std::collections::VecDeque<usize> = std::collections::VecDeque::with_capacity(w);
    let mut last: Option<usize> = None;
    for i in 0..n {
        while let Some(&b) = dq.back() {
            if hashes[b] >= hashes[i] {
                dq.pop_back();
            } else {
                break;
            }
        }
        dq.push_back(i);
        if let Some(&f) = dq.front() {
            if f + w <= i {
                dq.pop_front();
            }
        }
        if i + 1 >= w {
            let m = *dq.front().expect("non-empty");
            if last != Some(m) {
                out.push(Fingerprint { hash: hashes[m], pos: m as u32 });
                last = Some(m);
            }
        }
    }
    out
}

/// Distinct winnowed hashes of a token stream (what the index stores).
pub fn index_fingerprints(tokens: &[u64], k: usize, w: usize) -> Vec<u64> {
    let mut v: Vec<u64> = winnow(&kgram_hashes(tokens, k), w).into_iter().map(|f| f.hash).collect();
    v.sort_unstable();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::normalize;

    fn toks(s: &str) -> Vec<u64> {
        normalize(s, &[]).token_hashes()
    }

    /// Reference winnowing: brute force over every window.
    fn winnow_naive(h: &[u64], w: usize) -> Vec<Fingerprint> {
        let mut out: Vec<Fingerprint> = Vec::new();
        if h.len() <= w {
            if h.is_empty() {
                return out;
            }
            let (p, &m) = h.iter().enumerate().rev().min_by_key(|(_, x)| **x).unwrap();
            return vec![Fingerprint { hash: m, pos: p as u32 }];
        }
        for s in 0..=h.len() - w {
            let (p, &m) = h[s..s + w].iter().enumerate().rev().min_by_key(|(_, x)| **x).unwrap();
            let fp = Fingerprint { hash: m, pos: (s + p) as u32 };
            if out.last() != Some(&fp) {
                out.push(fp);
            }
        }
        out
    }

    #[test]
    fn kgrams_count_and_order_sensitivity() {
        let t = toks("a b c d e f");
        assert_eq!(kgram_hashes(&t, 4).len(), 3);
        assert!(kgram_hashes(&t, 7).is_empty());
        assert_ne!(hash_window(&toks("a b c")), hash_window(&toks("c b a")));
    }

    #[test]
    fn winnow_matches_naive() {
        let mut x = 0x1234_5678u64;
        for w in 1..8 {
            for n in 0..60 {
                let h: Vec<u64> = (0..n)
                    .map(|_| {
                        x = mix(x);
                        x % 17
                    })
                    .collect();
                assert_eq!(winnow(&h, w), winnow_naive(&h, w), "w={w} n={n}");
            }
        }
    }

    #[test]
    fn winnow_guarantee() {
        // Any shared run of >= w + k - 1 tokens shares a fingerprint.
        let (k, w) = (DEFAULT_K, DEFAULT_W);
        let shared = "the mitochondria is the powerhouse of the cell indeed";
        let a = toks(&format!("alpha beta gamma {shared} delta epsilon"));
        let b = toks(&format!("one two three four five six seven {shared} eight"));
        let fa = index_fingerprints(&a, k, w);
        let qb = kgram_hashes(&b, k);
        assert!(qb.iter().any(|h| fa.binary_search(h).is_ok()));
        // Density is roughly 2/(w+1).
        let long: Vec<u64> = (0..10_000u64).map(mix).collect();
        let d = winnow(&kgram_hashes(&long, k), w).len() as f64 / 10_000.0;
        assert!((0.4..0.6).contains(&d), "density {d}");
    }
}
