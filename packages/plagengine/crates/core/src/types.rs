//! Shared data model.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Half-open range `[start, end)` of **char** (Unicode scalar) indices.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "span {start}..{end}");
        Span { start, end }
    }
    pub fn len(&self) -> usize {
        self.end - self.start
    }
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }
    pub fn overlaps(&self, other: &Span) -> bool {
        self.start < other.end && other.start < self.end
    }
    pub fn intersect(&self, other: &Span) -> Option<Span> {
        let s = self.start.max(other.start);
        let e = self.end.min(other.end);
        (s < e).then(|| Span::new(s, e))
    }
}

/// Source collection a document belongs to (Turnitin's three repositories).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Collection {
    Internet,
    Publications,
    Submissions,
}

impl Collection {
    pub const ALL: [Collection; 3] = [Collection::Internet, Collection::Publications, Collection::Submissions];

    pub fn as_str(&self) -> &'static str {
        match self {
            Collection::Internet => "internet",
            Collection::Publications => "publications",
            Collection::Submissions => "submissions",
        }
    }
}

impl fmt::Display for Collection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Collection {
    type Err = crate::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "internet" | "web" => Ok(Collection::Internet),
            "publications" | "publication" | "pubs" => Ok(Collection::Publications),
            "submissions" | "submission" | "student" | "students" => Ok(Collection::Submissions),
            other => Err(crate::Error::Invalid(format!(
                "unknown collection '{other}' (expected internet, publications or submissions)"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocFormat {
    Txt,
    Md,
    Pdf,
    Docx,
}

impl DocFormat {
    pub fn from_path(path: &std::path::Path) -> Option<DocFormat> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "txt" | "text" => Some(DocFormat::Txt),
            "md" | "markdown" => Some(DocFormat::Md),
            "pdf" => Some(DocFormat::Pdf),
            "docx" => Some(DocFormat::Docx),
            _ => None,
        }
    }
}

/// Why a span of text is considered hidden from the reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HiddenReason {
    /// `w:vanish` / `w:specVanish` — hidden text property.
    Vanish,
    /// White or near-white font color.
    WhiteText,
    /// Font color equals the run shading / highlight color.
    MatchesBackground,
    /// Font size too small to read (≤ 2pt).
    TinyFont,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HiddenSpan {
    pub span: Span,
    pub reason: HiddenReason,
}

/// Output of the extract stage: the document's text plus layout facts that
/// later stages need. All spans are char ranges into `text`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ExtractedDoc {
    pub text: String,
    pub title: Option<String>,
    /// Text present in the file but invisible when rendered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden: Vec<HiddenSpan>,
    /// Block quotations (markdown `>` lines, docx Quote styles).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub block_quotes: Vec<Span>,
    /// Char positions of footnote/endnote references (citation evidence).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citation_markers: Vec<usize>,
}

impl ExtractedDoc {
    pub fn plain(text: impl Into<String>) -> Self {
        ExtractedDoc { text: text.into(), ..Default::default() }
    }

    pub fn hidden_spans(&self) -> Vec<Span> {
        self.hidden.iter().map(|h| h.span).collect()
    }
}

/// Collect chars of `chars[span]` into a String.
pub fn slice_chars(chars: &[char], span: Span) -> String {
    let e = span.end.min(chars.len());
    let s = span.start.min(e);
    chars[s..e].iter().collect()
}

/// Merge overlapping/adjacent spans; output sorted and disjoint.
pub fn merge_spans(mut spans: Vec<Span>) -> Vec<Span> {
    spans.retain(|s| !s.is_empty());
    spans.sort();
    let mut out: Vec<Span> = Vec::with_capacity(spans.len());
    for s in spans {
        match out.last_mut() {
            Some(last) if s.start <= last.end => last.end = last.end.max(s.end),
            _ => out.push(s),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_ops() {
        let a = Span::new(2, 6);
        let b = Span::new(5, 9);
        assert!(a.overlaps(&b));
        assert_eq!(a.intersect(&b), Some(Span::new(5, 6)));
        assert_eq!(a.intersect(&Span::new(6, 7)), None);
        assert!(a.contains(2) && !a.contains(6));
    }

    #[test]
    fn merge() {
        let m = merge_spans(vec![Span::new(5, 8), Span::new(0, 2), Span::new(2, 3), Span::new(7, 10), Span::new(4, 4)]);
        assert_eq!(m, vec![Span::new(0, 3), Span::new(5, 10)]);
    }

    #[test]
    fn collection_parse() {
        assert_eq!("Publications".parse::<Collection>().unwrap(), Collection::Publications);
        assert!("nope".parse::<Collection>().is_err());
        assert_eq!(serde_json::to_string(&Collection::Internet).unwrap(), "\"internet\"");
    }
}
