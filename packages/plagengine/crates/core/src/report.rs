//! Report building: a pure function of a raw [`CheckResult`] and
//! [`Filters`], so filters can be re-applied without re-checking.

use crate::attribute::{attribute, AttrParams, Attribution, Canvas};
use crate::context::{bibliography_span, citation_near, find_citations, quote_spans, Citation};
use crate::engine::{CheckResult, Passage};
use crate::flags::{detect_flags, Flags};
use crate::normalize::normalize;
use crate::types::{slice_chars, Collection, DocFormat, Span};
use serde::{Deserialize, Serialize};

/// Share of a match that must lie inside quotation marks to count as quoted.
const QUOTED_FRACTION: f64 = 0.8;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Filters {
    #[serde(default)]
    pub exclude_quotes: bool,
    #[serde(default)]
    pub exclude_bibliography: bool,
    /// Exclude matches smaller than N words (0 = off).
    #[serde(default)]
    pub min_words: usize,
    /// Exclude sources contributing less than N % (0 = off).
    #[serde(default)]
    pub min_percent: f64,
    #[serde(default)]
    pub exclude_sources: Vec<u32>,
    #[serde(default)]
    pub exclude_collections: Vec<Collection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatchGroup {
    #[serde(rename = "Not Cited or Quoted")]
    NotCitedOrQuoted,
    #[serde(rename = "Missing Quotations")]
    MissingQuotations,
    #[serde(rename = "Missing Citation")]
    MissingCitation,
    #[serde(rename = "Cited and Quoted")]
    CitedAndQuoted,
}

impl MatchGroup {
    pub const ALL: [MatchGroup; 4] =
        [MatchGroup::NotCitedOrQuoted, MatchGroup::MissingQuotations, MatchGroup::MissingCitation, MatchGroup::CitedAndQuoted];

    pub fn classify(quoted: bool, cited: bool) -> MatchGroup {
        match (quoted, cited) {
            (false, false) => MatchGroup::NotCitedOrQuoted,
            (false, true) => MatchGroup::MissingQuotations,
            (true, false) => MatchGroup::MissingCitation,
            (true, true) => MatchGroup::CitedAndQuoted,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubmissionSummary {
    pub title: String,
    pub path: String,
    pub format: DocFormat,
    pub characters: usize,
    pub non_space_chars: usize,
    pub countable_chars: usize,
    pub words: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Similarity {
    /// Integer percentage, as Turnitin displays it.
    pub percent: u32,
    pub exact: f64,
    pub matched_chars: usize,
    /// Countable chars after filters (the denominator).
    pub total_chars: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CollectionBreakdown {
    pub collection: Collection,
    /// Similarity against this collection alone (Turnitin's "Top Sources
    /// Overview" figure; the three can sum to more than the overall score).
    pub percent: f64,
    /// This collection's share of the overall score (sums to the overall).
    pub attributed_percent: f64,
    pub sources: usize,
    pub matches: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GroupCounts {
    #[serde(rename = "Not Cited or Quoted")]
    pub not_cited_or_quoted: usize,
    #[serde(rename = "Missing Quotations")]
    pub missing_quotations: usize,
    #[serde(rename = "Missing Citation")]
    pub missing_citation: usize,
    #[serde(rename = "Cited and Quoted")]
    pub cited_and_quoted: usize,
}

impl GroupCounts {
    fn add(&mut self, g: MatchGroup) {
        match g {
            MatchGroup::NotCitedOrQuoted => self.not_cited_or_quoted += 1,
            MatchGroup::MissingQuotations => self.missing_quotations += 1,
            MatchGroup::MissingCitation => self.missing_citation += 1,
            MatchGroup::CitedAndQuoted => self.cited_and_quoted += 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceEntry {
    pub rank: usize,
    pub source_id: u32,
    pub title: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub collection: Collection,
    /// Share of the submission attributed to this source.
    pub percent: f64,
    pub matched_chars: usize,
    pub match_count: usize,
    pub match_groups: GroupCounts,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MatchEntry {
    pub id: usize,
    pub source_id: u32,
    pub source_rank: usize,
    pub collection: Collection,
    pub text: String,
    pub submission_range: Span,
    pub source_range: Span,
    pub words: usize,
    pub chars: usize,
    pub group: MatchGroup,
    pub quoted: bool,
    pub cited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupSummary {
    pub group: MatchGroup,
    pub match_count: usize,
    pub chars: usize,
    pub percent: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Exclusions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bibliography: Option<Span>,
    pub quoted_ranges: usize,
    /// Countable chars removed from the denominator by filters.
    pub excluded_chars: usize,
    pub excluded_sources: Vec<u32>,
    pub excluded_collections: Vec<Collection>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub engine: String,
    pub check_id: String,
    pub created_unix: u64,
    pub submission: SubmissionSummary,
    pub filters: Filters,
    pub similarity: Similarity,
    pub by_collection: Vec<CollectionBreakdown>,
    pub match_groups: Vec<GroupSummary>,
    pub sources: Vec<SourceEntry>,
    pub matches: Vec<MatchEntry>,
    pub flags: Flags,
    pub excluded: Exclusions,
    /// The raw check, when requested, making the report re-filterable alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<Box<CheckResult>>,
}

fn pct(n: usize, d: usize) -> f64 {
    if d == 0 {
        0.0
    } else {
        (n as f64 * 10000.0 / d as f64).round() / 100.0
    }
}

/// Build the Turnitin-style report.
pub fn build_report(cr: &CheckResult, f: &Filters) -> Report {
    let doc = &cr.doc;
    let norm = normalize(&doc.text, &doc.hidden_spans());
    let chars = &norm.chars;
    let n = chars.len();
    let prefix = norm.countable_prefix();
    let countable_total = prefix[n] as usize;
    let single_nl = matches!(cr.submission.format, DocFormat::Docx);

    let quotes = quote_spans(chars, &doc.block_quotes, single_nl);
    let bib = bibliography_span(chars);
    let citations: Vec<Citation> = find_citations(chars, &doc.citation_markers);

    let mut blocked = vec![false; n];
    let mut block = |s: Span| blocked[s.start.min(n)..s.end.min(n)].iter_mut().for_each(|b| *b = true);
    if f.exclude_bibliography {
        if let Some(b) = bib {
            block(b);
        }
    }
    if f.exclude_quotes {
        quotes.iter().for_each(|q| block(*q));
    }
    let excluded_chars = (0..n).filter(|&i| blocked[i] && prefix[i + 1] > prefix[i]).count();
    let denominator = countable_total - excluded_chars;

    let collection_of = |sid: u32| cr.source(sid).map(|s| s.collection).unwrap_or(Collection::Internet);
    let passages: Vec<Passage> = cr
        .passages
        .iter()
        .filter(|p| !f.exclude_sources.contains(&p.source_id) && !f.exclude_collections.contains(&collection_of(p.source_id)))
        .cloned()
        .collect();

    let words = |s: Span| norm.tokens_in(s);
    let canvas = Canvas { countable_prefix: &prefix, blocked: &blocked, words: &words, denominator, quotes: &quotes };
    let params = AttrParams { min_words: f.min_words, min_percent: f.min_percent, ..Default::default() };
    let ranked: Vec<Attribution> = attribute(&passages, &canvas, &params);

    let quoted_count = |s: Span| -> usize {
        quotes.iter().filter_map(|q| q.intersect(&s)).map(|i| (prefix[i.end] - prefix[i.start]) as usize).sum()
    };

    let mut sources = Vec::new();
    let mut matches = Vec::new();
    for (ri, a) in ranked.iter().enumerate() {
        let info = cr.source(a.source_id);
        let collection = collection_of(a.source_id);
        let mut groups = GroupCounts::default();
        for piece in &a.pieces {
            let quoted = piece.chars > 0 && quoted_count(piece.sub) as f64 >= QUOTED_FRACTION * piece.chars as f64;
            let citation = citation_near(chars, piece.sub, &citations, single_nl);
            let group = MatchGroup::classify(quoted, citation.is_some());
            groups.add(group);
            matches.push(MatchEntry {
                id: 0,
                source_id: a.source_id,
                source_rank: ri + 1,
                collection,
                text: slice_chars(chars, piece.sub),
                submission_range: piece.sub,
                source_range: piece.src,
                words: norm.tokens_in(piece.sub),
                chars: piece.chars,
                group,
                quoted,
                cited: citation.is_some(),
                citation: citation.map(|c| slice_chars(chars, c.span)),
            });
        }
        sources.push(SourceEntry {
            rank: ri + 1,
            source_id: a.source_id,
            title: info.map(|s| s.title.clone()).unwrap_or_default(),
            path: info.map(|s| s.path.clone()).unwrap_or_default(),
            url: info.and_then(|s| s.url.clone()),
            collection,
            percent: pct(a.chars, denominator),
            matched_chars: a.chars,
            match_count: a.pieces.len(),
            match_groups: groups,
        });
    }
    matches.sort_by_key(|m| (m.submission_range.start, m.source_rank));
    for (i, m) in matches.iter_mut().enumerate() {
        m.id = i + 1;
    }
    let matched_chars: usize = ranked.iter().map(|a| a.chars).sum();

    let by_collection = Collection::ALL
        .iter()
        .map(|&c| {
            let only: Vec<Passage> = passages.iter().filter(|p| collection_of(p.source_id) == c).cloned().collect();
            let alone: usize = attribute(&only, &canvas, &params).iter().map(|a| a.chars).sum();
            let mine: Vec<&SourceEntry> = sources.iter().filter(|s| s.collection == c).collect();
            CollectionBreakdown {
                collection: c,
                percent: pct(alone, denominator),
                attributed_percent: pct(mine.iter().map(|s| s.matched_chars).sum(), denominator),
                sources: mine.len(),
                matches: mine.iter().map(|s| s.match_count).sum(),
            }
        })
        .collect();

    let match_groups = MatchGroup::ALL
        .iter()
        .map(|&g| {
            let ms: Vec<&MatchEntry> = matches.iter().filter(|m| m.group == g).collect();
            let chars: usize = ms.iter().map(|m| m.chars).sum();
            GroupSummary { group: g, match_count: ms.len(), chars, percent: pct(chars, denominator) }
        })
        .collect();

    let exact = pct(matched_chars, denominator);
    Report {
        engine: cr.engine.clone(),
        check_id: cr.check_id.clone(),
        created_unix: cr.created_unix,
        submission: SubmissionSummary {
            title: cr.submission.title.clone(),
            path: cr.submission.path.clone(),
            format: cr.submission.format,
            characters: n,
            non_space_chars: chars.iter().filter(|c| !c.is_whitespace()).count(),
            countable_chars: countable_total,
            words: norm.tokens.len(),
        },
        filters: f.clone(),
        similarity: Similarity { percent: exact.round() as u32, exact, matched_chars, total_chars: denominator },
        by_collection,
        match_groups,
        sources,
        matches,
        flags: detect_flags(&norm, &doc.hidden),
        excluded: Exclusions {
            bibliography: if f.exclude_bibliography { bib } else { None },
            quoted_ranges: if f.exclude_quotes { quotes.len() } else { 0 },
            excluded_chars,
            excluded_sources: f.exclude_sources.clone(),
            excluded_collections: f.exclude_collections.clone(),
        },
        raw: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidates::MemIndex;
    use crate::engine::{check_with_index, prepare, CheckOptions, SourceInfo, SubmissionInfo};
    use crate::store::StoreConfig;
    use crate::types::ExtractedDoc;

    const WEB: &str = "The Great Barrier Reef is the world's largest coral reef system, composed of over two thousand nine hundred individual reefs \
        and nine hundred islands stretching for over two thousand three hundred kilometres.";
    const JOURNAL: &str = "Coral bleaching occurs when corals expel the symbiotic algae living in their tissues, causing them to turn completely white, \
        and it is driven primarily by rising ocean temperatures linked to climate change.";
    const STUDENT: &str = "Marine protected areas have been shown to increase fish biomass substantially within a decade of their establishment \
        according to several long term monitoring programs in the region.";

    fn sources() -> Vec<(u32, Collection, &'static str)> {
        vec![(1, Collection::Internet, WEB), (2, Collection::Publications, JOURNAL), (3, Collection::Submissions, STUDENT)]
    }

    fn run_check(sub: &str) -> CheckResult {
        let cfg = StoreConfig::default();
        let mut idx = MemIndex::default();
        for (id, c, t) in sources() {
            idx.insert_doc(id, &prepare(ExtractedDoc::plain(t), "x.txt", c, cfg).fingerprints);
        }
        check_with_index(
            &idx,
            cfg.k,
            |id| {
                Ok(sources().into_iter().find(|s| s.0 == id).map(|(id, c, t)| {
                    (SourceInfo { id, title: format!("src{id}"), path: format!("src{id}.txt"), url: None, collection: c }, ExtractedDoc::plain(t))
                }))
            },
            ExtractedDoc::plain(sub),
            SubmissionInfo { title: "essay".into(), path: "essay.txt".into(), format: DocFormat::Txt },
            &CheckOptions::default(),
        )
        .unwrap()
    }

    fn essay() -> String {
        format!(
            "My essay opens with my own words about the ocean and why it matters to all of us today.\n\n\
             {WEB}\n\n\
             As one study notes, “{JOURNAL}” (Hughes et al., 2018).\n\n\
             Some research shows {STUDENT} (Smith, 2020).\n\n\
             Also, \"{}\" was said.\n\n\
             References\n\nHughes, T. et al. (2018). {}\n",
            &JOURNAL[..80],
            &WEB[..120]
        )
    }

    #[test]
    fn full_report_groups_and_breakdown() {
        let cr = run_check(&essay());
        let r = build_report(&cr, &Filters::default());
        assert_eq!(r.sources.len(), 3, "{:#?}", r.sources);
        assert!(r.similarity.percent > 30 && r.similarity.percent < 95, "{}", r.similarity.exact);
        // percentages of sources sum to overall (one char -> one source)
        let sum: usize = r.sources.iter().map(|s| s.matched_chars).sum();
        assert_eq!(sum, r.similarity.matched_chars);
        let by: std::collections::HashMap<Collection, &CollectionBreakdown> = r.by_collection.iter().map(|b| (b.collection, b)).collect();
        assert!(by[&Collection::Internet].percent > 0.0 && by[&Collection::Publications].percent > 0.0 && by[&Collection::Submissions].percent > 0.0);
        let attributed: f64 = r.by_collection.iter().map(|b| b.attributed_percent).sum();
        assert!((attributed - r.similarity.exact).abs() < 0.05);

        let group_of = |needle: &str| r.matches.iter().find(|m| m.text.contains(needle)).map(|m| m.group);
        assert_eq!(group_of("Great Barrier Reef is the world"), Some(MatchGroup::NotCitedOrQuoted));
        assert_eq!(group_of("symbiotic algae living"), Some(MatchGroup::CitedAndQuoted));
        assert_eq!(group_of("Marine protected areas"), Some(MatchGroup::MissingQuotations));
        assert!(r.matches.iter().any(|m| m.group == MatchGroup::MissingCitation), "{:#?}", r.matches);
        // Every match's source range points into the source text exactly.
        for m in &r.matches {
            let src = sources().into_iter().find(|s| s.0 == m.source_id).unwrap().2;
            let src_chars: Vec<char> = src.chars().collect();
            let a = normalize(&m.text, &[]).token_hashes();
            let b = normalize(&slice_chars(&src_chars, m.source_range), &[]).token_hashes();
            assert_eq!(a, b, "match {} maps exactly", m.id);
        }
        // Matches never overlap in the submission.
        for w in r.matches.windows(2) {
            assert!(w[0].submission_range.end <= w[1].submission_range.start);
        }
        let total: usize = r.match_groups.iter().map(|g| g.match_count).sum();
        assert_eq!(total, r.matches.len());
    }

    #[test]
    fn filters_rerun_without_recheck() {
        let cr = run_check(&essay());
        let base = build_report(&cr, &Filters::default());

        let q = build_report(&cr, &Filters { exclude_quotes: true, ..Default::default() });
        assert!(q.matches.iter().all(|m| !m.quoted));
        assert!(q.excluded.excluded_chars > 0);
        assert!(q.similarity.total_chars < base.similarity.total_chars);

        let b = build_report(&cr, &Filters { exclude_bibliography: true, ..Default::default() });
        let bib = b.excluded.bibliography.unwrap();
        assert!(b.matches.iter().all(|m| !m.submission_range.overlaps(&bib)));
        assert!(base.matches.iter().any(|m| m.submission_range.overlaps(&bib)));

        let s = build_report(&cr, &Filters { exclude_sources: vec![1], ..Default::default() });
        assert!(s.sources.iter().all(|x| x.source_id != 1));
        let c = build_report(&cr, &Filters { exclude_collections: vec![Collection::Submissions], ..Default::default() });
        assert!(c.sources.iter().all(|x| x.collection != Collection::Submissions));
        assert!(c.similarity.exact < base.similarity.exact);

        let w = build_report(&cr, &Filters { min_words: 25, ..Default::default() });
        assert!(w.matches.iter().all(|m| m.words >= 25));
        assert!(w.matches.len() < base.matches.len());

        let p = build_report(&cr, &Filters { min_percent: 50.0, ..Default::default() });
        assert!(p.sources.is_empty());
        // Unfiltered report is reproducible.
        assert_eq!(build_report(&cr, &Filters::default()), base);
    }
}
