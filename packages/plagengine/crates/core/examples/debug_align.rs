//! Debug helper: align two text files and print passages.
//! cargo run --release --example debug_align -- <susp.txt> <src.txt> [coarse_gap] [coarse_density]
use plagengine_core::align::{chain, fragments, AlignParams};
use plagengine_core::extract::extract_file;
use plagengine_core::normalize::normalize;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let q = normalize(&extract_file(args[1].as_ref()).unwrap().text, &[]);
    let s = normalize(&extract_file(args[2].as_ref()).unwrap().text, &[]);
    let mut p = AlignParams::default();
    if let Some(g) = args.get(3) {
        p.coarse_gap = g.parse().unwrap();
    }
    if let Some(d) = args.get(4) {
        p.coarse_min_density = d.parse().unwrap();
    }
    let (qh, sh) = (q.token_hashes(), s.token_hashes());
    let frags = fragments(&qh, &sh, p.k, p.max_occ);
    let window: Option<(usize, usize)> = args.get(5).map(|w| {
        let mut it = w.split('-').map(|x| x.parse::<usize>().unwrap());
        (it.next().unwrap(), it.next().unwrap())
    });
    let in_win = |c: usize| window.is_none_or(|(a, b)| c >= a && c < b);
    for f in &frags {
        if in_win(q.tokens[f.q].start) && f.len >= 3 {
            println!("frag q_tok {} s_tok {} len {}  (q char {})", f.q, f.s, f.len, q.tokens[f.q].start);
        }
    }
    for c in chain(&frags, &p) {
        if c.matched >= 4 && in_win(q.tokens[c.q_start].start) {
            println!(
                "passage q {}-{} s {}-{} matched {} density {:.2} chars q {}-{}",
                c.q_start, c.q_end, c.s_start, c.s_end, c.matched,
                c.matched as f64 / (c.q_end - c.q_start) as f64,
                q.tokens[c.q_start].start, q.tokens[c.q_end - 1].end
            );
        }
    }
}
