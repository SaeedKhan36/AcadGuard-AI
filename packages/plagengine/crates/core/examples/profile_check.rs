//! Timing breakdown of one check: cargo run --release --example profile_check -- <db> <file>
use plagengine_core::align::{align_indexed, chain, AlignParams, QueryIndex};
use plagengine_core::candidates::{select_candidates, CandidateParams};
use plagengine_core::fingerprint::kgram_hashes;
use plagengine_core::normalize::normalize;
use plagengine_core::Store;
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let store = Store::open(&a[1]).unwrap();
    let doc = plagengine_core::extract::extract_file(a[2].as_ref()).unwrap();
    let t = Instant::now();
    let q = normalize(&doc.text, &[]);
    let kg = kgram_hashes(&q.token_hashes(), store.config().k);
    println!("query tokens {} normalize {:?}", q.tokens.len(), t.elapsed());
    let t = Instant::now();
    let c = select_candidates(&store, &kg, &CandidateParams::default(), &[]).unwrap();
    println!("candidates {} in {:?}; window_hits top/bottom {:?} {:?}", c.len(), t.elapsed(), c.first().map(|x| x.window_hits), c.last().map(|x| x.window_hits));
    let p = AlignParams::default();
    let qi = QueryIndex::new(&q.token_hashes(), p.k, p.max_occ);
    let (mut tl, mut tn, mut ta, mut chars, mut found) = (0.0, 0.0, 0.0, 0usize, 0);
    let (mut tf, mut tc, mut nfr) = (0.0, 0.0, 0usize);
    for cand in &c {
        let t = Instant::now();
        let d = store.doc(cand.doc_id).unwrap().unwrap();
        tl += t.elapsed().as_secs_f64();
        let t = Instant::now();
        let s = normalize(&d.text, &[]);
        tn += t.elapsed().as_secs_f64();
        chars += s.chars.len();
        let t = Instant::now();
        let fr = qi.fragments(&s.token_hashes(), p.max_occ);
        tf += t.elapsed().as_secs_f64();
        nfr += fr.len();
        let t = Instant::now();
        let _ = chain(&fr, &p);
        tc += t.elapsed().as_secs_f64();
        let t = Instant::now();
        let p = align_indexed(&qi, &q, &s, &p);
        ta += t.elapsed().as_secs_f64();
        if !p.is_empty() {
            found += 1;
            println!("  match doc {} window_hits {} hits {} passages {}", cand.doc_id, cand.window_hits, cand.hits, p.len());
        }
    }
    println!("fragments {tf:.2}s ({nfr} frags) chain {tc:.2}s");
    println!("load {tl:.2}s normalize {tn:.2}s align {ta:.2}s over {chars} source chars; {found} sources matched");
}
