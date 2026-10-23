//! Document context used for Match Groups and filters: quotation spans,
//! in-text citations and the bibliography section. All functions are pure
//! and work on char indices.

use crate::types::{merge_spans, Span};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const MAX_QUOTE_CHARS: usize = 4000;

/// Is there a paragraph break at `i` (i.e. `chars[i]` is a newline that ends
/// a paragraph)? With `single_newline` every newline ends a paragraph (docx
/// paragraphs); otherwise a blank line is required (hard-wrapped text).
pub fn is_paragraph_break(chars: &[char], i: usize, single_newline: bool) -> bool {
    if chars[i] != '\n' {
        return false;
    }
    if single_newline {
        return true;
    }
    let mut j = i + 1;
    while j < chars.len() && chars[j] != '\n' && chars[j].is_whitespace() {
        j += 1;
    }
    (j < chars.len() && chars[j] == '\n') || {
        let mut b = i;
        while b > 0 && chars[b - 1] != '\n' && chars[b - 1].is_whitespace() {
            b -= 1;
        }
        b > 0 && chars[b - 1] == '\n'
    }
}

/// Spans of text inside quotation marks (marks excluded), merged with block
/// quotes.
pub fn quote_spans(chars: &[char], block_quotes: &[Span], single_newline: bool) -> Vec<Span> {
    let mut out: Vec<Span> = block_quotes.to_vec();
    // Stack of (closing char, content start).
    let mut open: Vec<(char, usize)> = Vec::new();
    let mut ascii_double: Option<usize> = None;
    let mut ascii_single: Option<usize> = None;
    let is_word = |c: char| c.is_alphanumeric();
    for i in 0..chars.len() {
        let c = chars[i];
        if c == '\n' && is_paragraph_break(chars, i, single_newline) {
            open.clear();
            ascii_double = None;
            ascii_single = None;
            continue;
        }
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let next = chars.get(i + 1).copied();
        // Closing marks first (a char can close before it could open).
        if let Some(pos) = open.iter().rposition(|(close, _)| *close == c) {
            let (_, start) = open[pos];
            open.truncate(pos);
            if i > start && i - start <= MAX_QUOTE_CHARS {
                out.push(Span::new(start, i));
            }
            continue;
        }
        match c {
            '“' => open.push(('”', i + 1)),
            '„' => open.push(('“', i + 1)),
            '«' => open.push(('»', i + 1)),
            '「' => open.push(('」', i + 1)),
            '『' => open.push(('』', i + 1)),
            '‘' if !prev.is_some_and(is_word) && next.is_some_and(|n| !n.is_whitespace()) => open.push(('’', i + 1)),
            '"' => match ascii_double.take() {
                Some(start) if i - start <= MAX_QUOTE_CHARS => out.push(Span::new(start, i)),
                Some(_) => {}
                None => ascii_double = Some(i + 1),
            },
            '\'' => {
                let opening = !prev.is_some_and(|p| is_word(p) || p == '\'') && next.is_some_and(is_word);
                let closing = !next.is_some_and(is_word) && prev.is_some_and(|p| !p.is_whitespace());
                match ascii_single {
                    Some(start) if closing => {
                        if i - start <= MAX_QUOTE_CHARS {
                            out.push(Span::new(start, i));
                        }
                        ascii_single = None;
                    }
                    None if opening => ascii_single = Some(i + 1),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    merge_spans(out)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationKind {
    /// (Smith, 2019), (Smith & Lee 2020, p. 4)
    AuthorDate,
    /// Smith (2019), Smith et al. (2020)
    Narrative,
    /// [12], [3, 5–7]
    Numeric,
    /// (Smith 42)
    Mla,
    /// Footnote/endnote reference or superscript number.
    Footnote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Citation {
    pub span: Span,
    pub kind: CitationKind,
}

fn regexes() -> &'static [(Regex, CitationKind)] {
    static RE: OnceLock<Vec<(Regex, CitationKind)>> = OnceLock::new();
    RE.get_or_init(|| {
        let year = r"(?:1[5-9]\d{2}|20\d{2})[a-z]?";
        let name = r"\p{Lu}[\p{L}'’\-]+";
        vec![
            (
                Regex::new(&format!(r"\((?:[^()]{{0,120}}?\p{{Lu}}[^()]{{0,120}}?(?:\b{year}\b|\bn\.d\.)[^()]{{0,60}})\)")).unwrap(),
                CitationKind::AuthorDate,
            ),
            (
                Regex::new(&format!(
                    r"\b{name}(?:\s+(?:et\s+al\.?|and|&)(?:\s+{name})?)?\s*\({year}(?:,\s*pp?\.\s*\d+(?:\s*[-–]\s*\d+)?)?\)"
                ))
                .unwrap(),
                CitationKind::Narrative,
            ),
            (Regex::new(r"\[\s*\d{1,3}(?:\s*[,;–\-]\s*\d{1,3})*\s*\]").unwrap(), CitationKind::Numeric),
            (
                Regex::new(&format!(r"\({name}(?:\s+(?:et al\.|and\s+{name}))?,?\s+\d{{1,4}}(?:\s*[-–]\s*\d{{1,4}})?\)")).unwrap(),
                CitationKind::Mla,
            ),
            (Regex::new(r"[\p{L}\p{P}][¹²³⁰⁴⁵⁶⁷⁸⁹]+").unwrap(), CitationKind::Footnote),
        ]
    })
}

/// All in-text citations in the document, sorted by position.
pub fn find_citations(chars: &[char], markers: &[usize]) -> Vec<Citation> {
    let text: String = chars.iter().collect();
    // byte offset -> char index
    let byte_to_char = {
        let mut v = vec![0usize; text.len() + 1];
        let mut ci = 0;
        for (bi, ch) in text.char_indices() {
            for b in 0..ch.len_utf8() {
                v[bi + b] = ci;
            }
            ci += 1;
        }
        v[text.len()] = ci;
        v
    };
    let mut out: Vec<Citation> = Vec::new();
    for (re, kind) in regexes() {
        for m in re.find_iter(&text) {
            out.push(Citation { span: Span::new(byte_to_char[m.start()], byte_to_char[m.end()]), kind: *kind });
        }
    }
    out.extend(markers.iter().map(|&p| Citation { span: Span::new(p, p + 1), kind: CitationKind::Footnote }));
    out.sort_by_key(|c| (c.span.start, c.span.end));
    out
}

fn is_sentence_end(chars: &[char], i: usize) -> bool {
    matches!(chars[i], '.' | '!' | '?') && chars.get(i + 1).is_none_or(|c| c.is_whitespace())
}

/// Window in which a citation counts for `piece`: from the start of the
/// sentence containing the piece's start through the end of the sentence
/// containing its end (bounded, never crossing a paragraph break).
pub fn citation_window(chars: &[char], piece: Span, single_newline: bool) -> Span {
    const BACK: usize = 400;
    const FWD: usize = 300;
    let lo = piece.start.saturating_sub(BACK);
    let mut start = piece.start;
    while start > lo {
        let i = start - 1;
        if is_paragraph_break(chars, i, single_newline) || is_sentence_end(chars, i) {
            break;
        }
        start -= 1;
    }
    let hi = (piece.end + FWD).min(chars.len());
    let mut end = piece.end;
    while end < hi {
        if is_paragraph_break(chars, end, single_newline) {
            break;
        }
        if is_sentence_end(chars, end) {
            end += 1;
            break;
        }
        end += 1;
    }
    Span::new(start, end)
}

/// A citation for `piece`: inside its sentence window, or immediately after
/// it (e.g. `"... end of quote." (Smith, 2019)`).
pub fn citation_near<'a>(chars: &[char], piece: Span, citations: &'a [Citation], single_newline: bool) -> Option<&'a Citation> {
    let w = citation_window(chars, piece, single_newline);
    let i = citations.partition_point(|c| c.span.end <= w.start);
    let c = citations.get(i)?;
    if c.span.start < w.end {
        return Some(c);
    }
    // Immediately following: only whitespace / closing quotes between.
    let between = &chars[w.end.min(c.span.start)..c.span.start];
    let adjacent = between.len() <= 6
        && between.iter().all(|&ch| (ch.is_whitespace() && ch != '\n') || matches!(ch, '"' | '”' | '’' | '\'' | '»'));
    adjacent.then_some(c)
}

const BIB_HEADINGS: &[&str] = &[
    "references", "reference list", "list of references", "references cited", "bibliography", "works cited",
    "works consulted", "literature cited", "cited works", "sources", "sources cited", "citations", "literature",
    "reference", "bibliografía", "literaturverzeichnis", "références",
];

fn heading_text(line: &[char]) -> String {
    let s: String = line.iter().collect();
    let s = s.trim().trim_start_matches('#').trim();
    // strip "7." / "VII." / "7 " numbering
    let s = s
        .trim_start_matches(|c: char| c.is_ascii_digit() || matches!(c, 'I' | 'V' | 'X'))
        .trim_start_matches(['.', ')'])
        .trim();
    s.trim_end_matches(':').trim().to_lowercase()
}

/// Locate the bibliography: the last References/Bibliography/... heading line
/// after the first 30% of the document, through the end (or to an Appendix
/// heading).
pub fn bibliography_span(chars: &[char]) -> Option<Span> {
    let n = chars.len();
    let mut lines: Vec<Span> = Vec::new();
    let mut s = 0;
    for (i, &c) in chars.iter().enumerate() {
        if c == '\n' {
            lines.push(Span::new(s, i));
            s = i + 1;
        }
    }
    lines.push(Span::new(s, n));
    let min_pos = n * 3 / 10;
    let start_line = lines.iter().rposition(|l| {
        l.start >= min_pos && l.len() <= 60 && BIB_HEADINGS.contains(&heading_text(&chars[l.start..l.end]).as_str())
    })?;
    let start = lines[start_line].start;
    let end = lines[start_line + 1..]
        .iter()
        .find(|l| {
            let h = heading_text(&chars[l.start..l.end]);
            l.len() <= 60 && (h.starts_with("appendix") || h.starts_with("appendices"))
        })
        .map(|l| l.start)
        .unwrap_or(n);
    Some(Span::new(start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(s: &str) -> Vec<char> {
        s.chars().collect()
    }
    fn texts(chars: &[char], spans: &[Span]) -> Vec<String> {
        spans.iter().map(|s| chars[s.start..s.end].iter().collect()).collect()
    }

    #[test]
    fn quotes_of_all_styles() {
        let t = c("He said “curly double” and \"straight double\" and ‘curly single’ and 'straight single'. Don't break on it's.");
        let q = quote_spans(&t, &[], false);
        assert_eq!(texts(&t, &q), vec!["curly double", "straight double", "curly single", "straight single"]);
    }

    #[test]
    fn quotes_do_not_cross_paragraphs_and_include_blocks() {
        let t = c("An \"unclosed quote here\n\nand \"closed one\" there");
        assert_eq!(texts(&t, &quote_spans(&t, &[], false)), vec!["closed one"]);
        let q = quote_spans(&t, &[Span::new(0, 2)], false);
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn possessive_apostrophes_are_not_quotes() {
        let t = c("The students' papers and the teachers' notes were the authors' work.");
        assert!(quote_spans(&t, &[], false).is_empty());
    }

    #[test]
    fn citation_styles() {
        let t = c("As shown (Smith & Lee, 2019, p. 4) and Jones et al. (2020) found, see [3, 5-7] and (Brown 42) or note² and (WHO, n.d.).");
        let kinds: Vec<CitationKind> = find_citations(&t, &[]).iter().map(|c| c.kind).collect();
        assert_eq!(
            kinds,
            vec![CitationKind::AuthorDate, CitationKind::Narrative, CitationKind::Numeric, CitationKind::Mla, CitationKind::Footnote, CitationKind::AuthorDate]
        );
        assert!(find_citations(&c("It happened in 1999 (in the summer of 1999)."), &[]).is_empty());
    }

    #[test]
    fn citation_window_and_near() {
        let t = c("First sentence is mine. The copied passage is here and continues on (Smith, 2019). Next sentence.\n\nOther para (Doe, 2001).");
        let cites = find_citations(&t, &[]);
        let piece = Span::new(24, 68);
        assert!(citation_near(&t, piece, &cites, false).is_some());
        // The citation belongs to the next sentence, not the first.
        assert!(citation_near(&t, Span::new(0, 22), &cites, false).is_none());
        // Citation right after the closing quote of a sentence counts.
        let t2 = c("He wrote “this exact copied sentence.” (Smith, 2019) Then more.");
        let cites2 = find_citations(&t2, &[]);
        assert!(citation_near(&t2, Span::new(10, 36), &cites2, false).is_some());
    }

    #[test]
    fn bibliography_detection() {
        let body = "Lorem ipsum dolor sit amet. ".repeat(20);
        let t = c(&format!("References in the intro do not count\n{body}\n\nReferences\nSmith, J. (2019). A book.\n\nAppendix A\nData"));
        let b = bibliography_span(&t).unwrap();
        let text: String = t[b.start..b.end].iter().collect();
        assert!(text.starts_with("References\nSmith"));
        assert!(text.trim_end().ends_with("A book."));
        let t2 = c(&format!("{body}\n## 7. Works Cited:\nfoo"));
        assert!(bibliography_span(&t2).is_some());
        assert!(bibliography_span(&c(&body)).is_none());
    }
}
