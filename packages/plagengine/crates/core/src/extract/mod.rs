//! Stage 0 — text extraction from .txt, .md, .pdf and .docx.
//!
//! Text is kept verbatim for plain formats so char offsets line up with the
//! file (PAN annotations rely on this).

pub mod docx;

use crate::types::{DocFormat, ExtractedDoc, Span};
use crate::{Error, Result};
use std::path::Path;

/// Extract a file, choosing the format from its extension.
pub fn extract_file(path: &Path) -> Result<ExtractedDoc> {
    let format = DocFormat::from_path(path).ok_or_else(|| Error::UnsupportedFormat(path.display().to_string()))?;
    let bytes = std::fs::read(path).map_err(|e| Error::Io { path: path.display().to_string(), source: e })?;
    extract_bytes(&bytes, format).map_err(|e| match e {
        Error::Extract { msg, .. } => Error::Extract { path: path.display().to_string(), msg },
        other => other,
    })
}

pub fn extract_bytes(bytes: &[u8], format: DocFormat) -> Result<ExtractedDoc> {
    match format {
        DocFormat::Txt => Ok(ExtractedDoc::plain(decode_text(bytes))),
        DocFormat::Md => Ok(extract_markdown(&decode_text(bytes))),
        DocFormat::Pdf => extract_pdf(bytes),
        DocFormat::Docx => docx::extract_docx(bytes),
    }
}

/// UTF-8 (a leading BOM is stripped, so offsets match PAN annotations),
/// falling back to Windows-1252 for legacy files.
pub fn decode_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| cp1252(b)).collect(),
    }
}

fn cp1252(b: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž', '\u{8F}', '\u{90}', '‘', '’', '“',
        '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}', 'ž', 'Ÿ',
    ];
    match b {
        0x80..=0x9F => HIGH[(b - 0x80) as usize],
        _ => b as char,
    }
}

/// Markdown is kept verbatim (markup chars are not tokens); `>` lines are
/// recorded as block quotes.
pub fn extract_markdown(text: &str) -> ExtractedDoc {
    let mut block_quotes = Vec::new();
    let mut pos = 0usize;
    let mut title = None;
    for line in text.split_inclusive('\n') {
        let n = line.chars().count();
        let trimmed = line.trim_start();
        if trimmed.starts_with('>') {
            let lead = line.chars().count() - trimmed.chars().count() + 1;
            let content_end = line.trim_end().chars().count();
            if content_end > lead {
                block_quotes.push(Span::new(pos + lead, pos + content_end));
            }
        }
        if title.is_none() {
            if let Some(h) = trimmed.strip_prefix("# ") {
                title = Some(h.trim().to_string());
            }
        }
        pos += n;
    }
    ExtractedDoc { text: text.to_string(), title, block_quotes: crate::types::merge_spans(block_quotes), ..Default::default() }
}

/// PDF text via `pdf-extract`. The library can panic on malformed files, so
/// it runs under `catch_unwind`.
pub fn extract_pdf(bytes: &[u8]) -> Result<ExtractedDoc> {
    let owned = bytes.to_vec();
    let res = std::panic::catch_unwind(move || pdf_extract::extract_text_from_mem(&owned));
    match res {
        Ok(Ok(text)) => Ok(ExtractedDoc::plain(text)),
        Ok(Err(e)) => Err(Error::Extract { path: String::new(), msg: e.to_string() }),
        Err(_) => Err(Error::Extract { path: String::new(), msg: "pdf parser panicked".into() }),
    }
}

/// Test helpers shared with integration tests.
#[doc(hidden)]
pub mod testing {
    /// Build a minimal single-page PDF (Helvetica, one text line per entry).
    pub fn minimal_pdf(lines: &[&str]) -> Vec<u8> {
        let mut content = String::from("BT /F1 12 Tf 72 720 Td 14 TL\n");
        for l in lines {
            content.push_str(&format!("({}) Tj T*\n", l.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")));
        }
        content.push_str("ET");
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
            format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), content),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
        ];
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, o) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", i + 1, o).as_bytes());
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
        for off in offsets {
            out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::testing::minimal_pdf;
    use super::*;

    #[test]
    fn markdown_block_quotes_and_title() {
        let md = "# My Essay\n\nIntro text.\n> quoted line one\n> quoted line two\nAfter.";
        let d = extract_markdown(md);
        assert_eq!(d.title.as_deref(), Some("My Essay"));
        assert_eq!(d.block_quotes.len(), 2);
        let chars: Vec<char> = md.chars().collect();
        let q: String = chars[d.block_quotes[0].start..d.block_quotes[0].end].iter().collect();
        assert_eq!(q, " quoted line one");
        assert_eq!(d.text, md);
    }

    #[test]
    fn legacy_encoding_fallback() {
        assert_eq!(decode_text(b"caf\xe9 \x93hi\x94"), "café “hi”");
        assert_eq!(decode_text("ok ✓".as_bytes()), "ok ✓");
        assert_eq!(decode_text("\u{FEFF}bom".as_bytes()), "bom");
    }

    #[test]
    fn pdf_text_is_extracted() {
        let pdf = minimal_pdf(&["Plagiarism detection engines compare documents.", "Second line (with parens)."]);
        let d = extract_pdf(&pdf).unwrap();
        let words: Vec<&str> = d.text.split_whitespace().collect();
        assert_eq!(words, vec!["Plagiarism", "detection", "engines", "compare", "documents.", "Second", "line", "(with", "parens)."]);
    }

    #[test]
    fn bad_pdf_is_an_error_not_a_panic() {
        assert!(extract_pdf(b"%PDF-1.4 garbage").is_err());
    }
}
