//! .docx extraction with run formatting, so hidden text can be flagged.
//!
//! Effective run properties are resolved as: docDefaults → paragraph style
//! chain → character style chain → direct `w:rPr`. A run is hidden when it
//! has `w:vanish`/`w:specVanish`, a white/near-white (or `background1`)
//! color without a dark highlight/shading behind it, a color equal to its
//! shading/highlight, or a font size ≤ 2pt.
//!
//! Also provides [`DocxBuilder`], a minimal writer used by tests and the
//! adversarial evaluation set.

use crate::types::{ExtractedDoc, HiddenReason, HiddenSpan, Span};
use crate::{Error, Result};
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

#[derive(Clone, Debug, Default, PartialEq)]
struct RPr {
    vanish: Option<bool>,
    color: Option<String>,       // uppercase hex, or "AUTO"
    theme_color: Option<String>, // lowercase theme name
    size_half_pts: Option<u32>,
    highlight: Option<String>, // hex
    shading: Option<String>,   // hex fill
    rstyle: Option<String>,
}

impl RPr {
    fn overlay(&self, over: &RPr) -> RPr {
        RPr {
            vanish: over.vanish.or(self.vanish),
            color: over.color.clone().or_else(|| self.color.clone()),
            theme_color: if over.color.is_some() || over.theme_color.is_some() {
                over.theme_color.clone()
            } else {
                self.theme_color.clone()
            },
            size_half_pts: over.size_half_pts.or(self.size_half_pts),
            highlight: over.highlight.clone().or_else(|| self.highlight.clone()),
            shading: over.shading.clone().or_else(|| self.shading.clone()),
            rstyle: over.rstyle.clone().or_else(|| self.rstyle.clone()),
        }
    }

    fn hidden_reason(&self) -> Option<HiddenReason> {
        if self.vanish == Some(true) {
            return Some(HiddenReason::Vanish);
        }
        if self.size_half_pts.is_some_and(|s| s <= 4) {
            return Some(HiddenReason::TinyFont);
        }
        let background = self.highlight.clone().or_else(|| self.shading.clone()).filter(|b| b != "AUTO");
        let color = self.color.clone().filter(|c| c != "AUTO");
        let theme_white = self
            .theme_color
            .as_deref()
            .is_some_and(|t| matches!(t, "background1" | "bg1" | "light1" | "lt1"));
        if let (Some(c), Some(b)) = (&color, &background) {
            if c == b {
                return Some(HiddenReason::MatchesBackground);
            }
        }
        let white = theme_white || color.as_deref().is_some_and(is_near_white);
        if white && background.as_deref().is_none_or(is_near_white) {
            return Some(HiddenReason::WhiteText);
        }
        None
    }
}

fn is_near_white(hex: &str) -> bool {
    hex.len() == 6
        && (0..3).all(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).is_ok_and(|v| v >= 0xF0))
}

fn highlight_hex(name: &str) -> Option<String> {
    Some(
        match name {
            "black" => "000000",
            "white" => "FFFFFF",
            "yellow" => "FFFF00",
            "green" => "00FF00",
            "cyan" => "00FFFF",
            "magenta" => "FF00FF",
            "blue" => "0000FF",
            "red" => "FF0000",
            "darkBlue" => "000080",
            "darkCyan" => "008080",
            "darkGreen" => "008000",
            "darkMagenta" => "800080",
            "darkRed" => "800000",
            "darkYellow" => "808000",
            "darkGray" => "808080",
            "lightGray" => "C0C0C0",
            _ => return None,
        }
        .to_string(),
    )
}

fn attr(e: &BytesStart, local: &str) -> Option<String> {
    e.attributes().flatten().find(|a| a.key.local_name().as_ref() == local).and_then(|a| {
        a.normalized_value(XmlVersion::Implicit1_0).ok().map(|v| v.into_owned())
    })
}

fn on_off(e: &BytesStart) -> bool {
    !matches!(attr(e, "val").as_deref(), Some("0" | "false" | "off" | "none"))
}

/// Apply an `w:rPr` child element to `r`.
fn apply_rpr_child(r: &mut RPr, e: &BytesStart) {
    match e.local_name().as_ref() {
        "vanish" | "specVanish" => r.vanish = Some(on_off(e)),
        "color" => {
            r.color = attr(e, "val").map(|v| v.to_ascii_uppercase());
            r.theme_color = attr(e, "themeColor").map(|v| v.to_ascii_lowercase());
        }
        "sz" => r.size_half_pts = attr(e, "val").and_then(|v| v.parse().ok()),
        "highlight" => r.highlight = attr(e, "val").and_then(|v| if v == "none" { None } else { highlight_hex(&v) }),
        "shd" => r.shading = attr(e, "fill").map(|v| v.to_ascii_uppercase()).filter(|v| v != "AUTO"),
        "rStyle" => r.rstyle = attr(e, "val"),
        _ => {}
    }
}

#[derive(Clone, Debug, Default)]
struct StyleDef {
    name: String,
    based_on: Option<String>,
    rpr: RPr,
}

#[derive(Debug, Default)]
struct Styles {
    defaults: RPr,
    styles: HashMap<String, StyleDef>,
    default_paragraph: Option<String>,
}

impl Styles {
    fn resolve(&self, id: &str, depth: usize) -> RPr {
        let Some(s) = self.styles.get(id) else { return RPr::default() };
        let base = match (&s.based_on, depth < 16) {
            (Some(b), true) => self.resolve(b, depth + 1),
            _ => RPr::default(),
        };
        base.overlay(&s.rpr)
    }
    fn is_quote_style(&self, id: &str) -> bool {
        let name = self.styles.get(id).map(|s| s.name.to_lowercase()).unwrap_or_default();
        let id = id.to_lowercase();
        name.contains("quote") || id.contains("quote") || name == "block text"
    }
}

fn parse_styles(xml: &str) -> Styles {
    let mut st = Styles::default();
    let mut reader = Reader::from_str(xml);
    let mut cur: Option<(String, StyleDef, bool)> = None; // (id, def, is_default_paragraph)
    let mut in_defaults_rpr = false;
    let mut in_rpr = false;
    loop {
        let ev = match reader.read_event() {
            Ok(Event::Eof) | Err(_) => break,
            Ok(ev) => ev,
        };
        match ev {
            Event::Start(e) | Event::Empty(e) => {
                let local = e.local_name();
                match local.as_ref() {
                    "style" => {
                        let id = attr(&e, "styleId").unwrap_or_default();
                        let is_def = attr(&e, "type").as_deref() == Some("paragraph") && attr(&e, "default").as_deref() == Some("1");
                        cur = Some((id, StyleDef::default(), is_def));
                    }
                    "name" => {
                        if let Some((_, d, _)) = cur.as_mut() {
                            d.name = attr(&e, "val").unwrap_or_default();
                        }
                    }
                    "basedOn" => {
                        if let Some((_, d, _)) = cur.as_mut() {
                            d.based_on = attr(&e, "val");
                        }
                    }
                    "rPrDefault" => in_defaults_rpr = true,
                    "rPr" => in_rpr = true,
                    _ if in_rpr => {
                        if in_defaults_rpr {
                            apply_rpr_child(&mut st.defaults, &e);
                        } else if let Some((_, d, _)) = cur.as_mut() {
                            apply_rpr_child(&mut d.rpr, &e);
                        }
                    }
                    _ => {}
                }
            }
            Event::End(e) => match e.local_name().as_ref() {
                "style" => {
                    if let Some((id, d, is_def)) = cur.take() {
                        if is_def {
                            st.default_paragraph = Some(id.clone());
                        }
                        st.styles.insert(id, d);
                    }
                }
                "rPrDefault" => in_defaults_rpr = false,
                "rPr" => in_rpr = false,
                _ => {}
            },
            _ => {}
        }
    }
    st
}

/// Accumulates text with char offsets and hidden spans.
#[derive(Default)]
struct TextBuilder {
    text: String,
    len: usize,
    hidden: Vec<HiddenSpan>,
    block_quotes: Vec<Span>,
    markers: Vec<usize>,
}

impl TextBuilder {
    fn push(&mut self, s: &str, hidden: Option<HiddenReason>) {
        let n = s.chars().count();
        if n == 0 {
            return;
        }
        if let Some(reason) = hidden {
            match self.hidden.last_mut() {
                Some(h) if h.span.end == self.len && h.reason == reason => h.span.end += n,
                _ => self.hidden.push(HiddenSpan { span: Span::new(self.len, self.len + n), reason }),
            }
        }
        self.text.push_str(s);
        self.len += n;
    }
}

struct Para {
    style: Option<String>,
    start: usize,
}

/// Parse a WordprocessingML part (document, footnotes, endnotes).
fn parse_part(xml: &str, styles: &Styles, out: &mut TextBuilder) {
    let mut reader = Reader::from_str(xml);
    let mut paras: Vec<Para> = Vec::new();
    // Per open run: direct props.
    let mut runs: Vec<RPr> = Vec::new();
    let mut in_ppr = false;
    let mut in_rpr = false;
    let mut in_text = false; // inside w:t
    let mut skip_depth = 0usize; // inside w:delText / w:instrText

    let effective = |paras: &[Para], runs: &[RPr]| -> Option<HiddenReason> {
        let mut r = styles.defaults.clone();
        let pstyle = paras.last().and_then(|p| p.style.clone()).or_else(|| styles.default_paragraph.clone());
        if let Some(ps) = pstyle {
            r = r.overlay(&styles.resolve(&ps, 0));
        }
        if let Some(direct) = runs.last() {
            if let Some(rs) = &direct.rstyle {
                r = r.overlay(&styles.resolve(rs, 0));
            }
            r = r.overlay(direct);
        }
        r.hidden_reason()
    };

    loop {
        let ev = match reader.read_event() {
            Ok(Event::Eof) | Err(_) => break,
            Ok(ev) => ev,
        };
        match ev {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let is_empty = matches!(ev, Event::Empty(_));
                let local = e.local_name();
                match local.as_ref() {
                    "p" if !is_empty => paras.push(Para { style: None, start: out.len }),
                    "p" => out.push("\n", None),
                    "pPr" if !is_empty => in_ppr = true,
                    "pStyle" if in_ppr => {
                        if let Some(p) = paras.last_mut() {
                            p.style = attr(e, "val");
                        }
                    }
                    "r" if !is_empty => runs.push(RPr::default()),
                    "rPr" if !in_ppr && !is_empty && !runs.is_empty() => in_rpr = true,
                    _ if in_rpr => {
                        if let Some(r) = runs.last_mut() {
                            apply_rpr_child(r, e);
                        }
                    }
                    "t" if !is_empty => in_text = true,
                    "delText" | "instrText" if !is_empty => skip_depth += 1,
                    "tab" if !in_ppr && !runs.is_empty() => out.push("\t", effective(&paras, &runs)),
                    "br" | "cr" if !runs.is_empty() => out.push("\n", None),
                    "noBreakHyphen" => out.push("-", effective(&paras, &runs)),
                    "footnoteReference" | "endnoteReference" => out.markers.push(out.len),
                    _ => {}
                }
            }
            Event::End(e) => match e.local_name().as_ref() {
                "p" => {
                    if let Some(p) = paras.pop() {
                        if p.style.as_deref().is_some_and(|s| styles.is_quote_style(s)) && out.len > p.start {
                            out.block_quotes.push(Span::new(p.start, out.len));
                        }
                    }
                    out.push("\n", None);
                }
                "pPr" => in_ppr = false,
                "rPr" => in_rpr = false,
                "r" => {
                    runs.pop();
                }
                "t" => in_text = false,
                "delText" | "instrText" => skip_depth = skip_depth.saturating_sub(1),
                _ => {}
            },
            Event::Text(t) if in_text && skip_depth == 0 => {
                let s = t.xml10_content();
                out.push(&s, effective(&paras, &runs));
            }
            Event::GeneralRef(r) if in_text && skip_depth == 0 => {
                let s = match r.resolve_char_ref() {
                    Ok(Some(c)) => c.to_string(),
                    _ => quick_xml::escape::resolve_predefined_entity(&r.xml10_content()).unwrap_or("").to_string(),
                };
                out.push(&s, effective(&paras, &runs));
            }
            _ => {}
        }
    }
}

fn read_entry(zip: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> Option<String> {
    let mut f = zip.by_name(name).ok()?;
    let mut s = String::new();
    f.read_to_string(&mut s).ok()?;
    Some(s)
}

fn core_title(xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    let mut in_title = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if e.local_name().as_ref() == "title" => in_title = true,
            Ok(Event::Text(t)) if in_title => {
                let s = t.xml10_content().trim().to_string();
                return (!s.is_empty()).then_some(s);
            }
            Ok(Event::End(_)) => in_title = false,
            Ok(Event::Eof) | Err(_) => return None,
            _ => {}
        }
    }
}

/// Extract a .docx from bytes.
pub fn extract_docx(bytes: &[u8]) -> Result<ExtractedDoc> {
    let err = |msg: String| Error::Extract { path: String::new(), msg };
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| err(format!("not a zip/docx: {e}")))?;
    let document = read_entry(&mut zip, "word/document.xml").ok_or_else(|| err("missing word/document.xml".into()))?;
    let styles = read_entry(&mut zip, "word/styles.xml").map(|s| parse_styles(&s)).unwrap_or_default();
    let mut out = TextBuilder::default();
    parse_part(&document, &styles, &mut out);
    for part in ["word/footnotes.xml", "word/endnotes.xml"] {
        if let Some(xml) = read_entry(&mut zip, part) {
            let before = out.len;
            let mut notes = TextBuilder { len: out.len, ..Default::default() };
            parse_part(&xml, &styles, &mut notes);
            if notes.text.chars().any(|c| !c.is_whitespace()) {
                out.text.push_str(&notes.text);
                out.len = before + notes.text.chars().count();
                out.hidden.extend(notes.hidden);
                out.block_quotes.extend(notes.block_quotes);
            }
        }
    }
    let title = read_entry(&mut zip, "docProps/core.xml").and_then(|s| core_title(&s));
    Ok(ExtractedDoc {
        text: out.text,
        title,
        hidden: out.hidden,
        block_quotes: out.block_quotes,
        citation_markers: out.markers,
    })
}

// ---------------------------------------------------------------- writer

/// One run for [`DocxBuilder`].
#[derive(Clone, Debug, Default)]
pub struct RunSpec {
    pub text: String,
    /// Hex color, e.g. "FFFFFF".
    pub color: Option<String>,
    pub size_half_pts: Option<u32>,
    pub vanish: bool,
}

impl RunSpec {
    pub fn plain(t: impl Into<String>) -> Self {
        RunSpec { text: t.into(), ..Default::default() }
    }
    pub fn white(t: impl Into<String>) -> Self {
        RunSpec { text: t.into(), color: Some("FFFFFF".into()), ..Default::default() }
    }
    pub fn tiny(t: impl Into<String>) -> Self {
        RunSpec { text: t.into(), size_half_pts: Some(2), ..Default::default() }
    }
    pub fn vanish(t: impl Into<String>) -> Self {
        RunSpec { text: t.into(), vanish: true, ..Default::default() }
    }
}

/// Minimal .docx writer.
#[derive(Default)]
pub struct DocxBuilder {
    paragraphs: Vec<(Option<String>, Vec<RunSpec>)>,
    title: Option<String>,
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

impl DocxBuilder {
    pub fn title(mut self, t: &str) -> Self {
        self.title = Some(t.to_string());
        self
    }
    pub fn paragraph(mut self, runs: Vec<RunSpec>) -> Self {
        self.paragraphs.push((None, runs));
        self
    }
    pub fn styled_paragraph(mut self, style: &str, runs: Vec<RunSpec>) -> Self {
        self.paragraphs.push((Some(style.to_string()), runs));
        self
    }

    pub fn build(&self) -> Vec<u8> {
        let mut body = String::new();
        for (style, runs) in &self.paragraphs {
            body.push_str("<w:p>");
            if let Some(s) = style {
                body.push_str(&format!("<w:pPr><w:pStyle w:val=\"{}\"/></w:pPr>", esc(s)));
            }
            for r in runs {
                body.push_str("<w:r>");
                let mut rpr = String::new();
                if r.vanish {
                    rpr.push_str("<w:vanish/>");
                }
                if let Some(c) = &r.color {
                    rpr.push_str(&format!("<w:color w:val=\"{c}\"/>"));
                }
                if let Some(sz) = r.size_half_pts {
                    rpr.push_str(&format!("<w:sz w:val=\"{sz}\"/>"));
                }
                if !rpr.is_empty() {
                    body.push_str(&format!("<w:rPr>{rpr}</w:rPr>"));
                }
                body.push_str(&format!("<w:t xml:space=\"preserve\">{}</w:t></w:r>", esc(&r.text)));
            }
            body.push_str("</w:p>");
        }
        let document = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
             <w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>{body}</w:body></w:document>"
        );
        let styles = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
            <w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
            <w:docDefaults><w:rPrDefault><w:rPr><w:sz w:val=\"24\"/></w:rPr></w:rPrDefault></w:docDefaults>\
            <w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style>\
            <w:style w:type=\"paragraph\" w:styleId=\"Quote\"><w:name w:val=\"Quote\"/><w:basedOn w:val=\"Normal\"/></w:style>\
            </w:styles>";
        let content_types = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
            <Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
            <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
            <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
            <Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
            <Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
            <Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>\
            </Types>";
        let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
            <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
            <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/>\
            <Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/>\
            </Relationships>";
        let doc_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
            <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
            <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>\
            </Relationships>";
        let core = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
             <cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" \
             xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><dc:title>{}</dc:title></cp:coreProperties>",
            esc(self.title.as_deref().unwrap_or(""))
        );
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            for (name, data) in [
                ("[Content_Types].xml", content_types.to_string()),
                ("_rels/.rels", rels.to_string()),
                ("word/_rels/document.xml.rels", doc_rels.to_string()),
                ("word/document.xml", document),
                ("word/styles.xml", styles.to_string()),
                ("docProps/core.xml", core),
            ] {
                zw.start_file(name, opts).expect("zip write");
                zw.write_all(data.as_bytes()).expect("zip write");
            }
            zw.finish().expect("zip finish");
        }
        buf.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(d: &ExtractedDoc, s: Span) -> String {
        d.text.chars().skip(s.start).take(s.len()).collect()
    }

    #[test]
    fn roundtrip_with_hidden_runs() {
        let bytes = DocxBuilder::default()
            .title("Essay")
            .paragraph(vec![RunSpec::plain("The quick"), RunSpec::white("x"), RunSpec::plain("brown fox & friends.")])
            .paragraph(vec![RunSpec::plain("Visible "), RunSpec::vanish("secret"), RunSpec::tiny("tiny"), RunSpec::plain(" end.")])
            .styled_paragraph("Quote", vec![RunSpec::plain("A block quotation.")])
            .build();
        let d = extract_docx(&bytes).unwrap();
        assert_eq!(d.title.as_deref(), Some("Essay"));
        assert_eq!(d.text, "The quickxbrown fox & friends.\nVisible secrettiny end.\nA block quotation.\n");
        let hidden: Vec<(String, HiddenReason)> = d.hidden.iter().map(|h| (text_of(&d, h.span), h.reason)).collect();
        assert_eq!(
            hidden,
            vec![
                ("x".to_string(), HiddenReason::WhiteText),
                ("secret".to_string(), HiddenReason::Vanish),
                ("tiny".to_string(), HiddenReason::TinyFont)
            ]
        );
        assert_eq!(d.block_quotes.len(), 1);
        assert_eq!(text_of(&d, d.block_quotes[0]), "A block quotation.");
    }

    #[test]
    fn style_resolution_and_backgrounds() {
        let styles = r#"<w:styles xmlns:w="w">
            <w:docDefaults><w:rPrDefault><w:rPr><w:sz w:val="24"/></w:rPr></w:rPrDefault></w:docDefaults>
            <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
            <w:style w:type="character" w:styleId="Ghost"><w:name w:val="Ghost"/><w:rPr><w:color w:val="FAFAFA"/></w:rPr></w:style>
            <w:style w:type="character" w:styleId="Ghost2"><w:basedOn w:val="Ghost"/></w:style>
        </w:styles>"#;
        let st = parse_styles(styles);
        let doc = r#"<w:document xmlns:w="w"><w:body><w:p>
            <w:r><w:rPr><w:rStyle w:val="Ghost2"/></w:rPr><w:t>styled</w:t></w:r>
            <w:r><w:rPr><w:color w:val="FFFFFF"/><w:highlight w:val="black"/></w:rPr><w:t>onblack</w:t></w:r>
            <w:r><w:rPr><w:color w:val="112233"/><w:shd w:fill="112233"/></w:rPr><w:t>same</w:t></w:r>
            <w:r><w:rPr><w:color w:val="FFFFFF" w:themeColor="background1"/></w:rPr><w:t>theme</w:t></w:r>
            <w:r><w:rPr><w:vanish w:val="0"/></w:rPr><w:t>shown</w:t></w:r>
            <w:r><w:t>a&amp;b&#x41;</w:t></w:r>
            <w:r><w:delText>deleted</w:delText><w:instrText>PAGE</w:instrText></w:r>
            <w:r><w:footnoteReference w:id="1"/></w:r>
        </w:p></w:body></w:document>"#;
        let mut out = TextBuilder::default();
        parse_part(doc, &st, &mut out);
        assert_eq!(out.text, "styledonblacksamethemeshowna&bA\n");
        let reasons: Vec<HiddenReason> = out.hidden.iter().map(|h| h.reason).collect();
        assert_eq!(reasons, vec![HiddenReason::WhiteText, HiddenReason::MatchesBackground, HiddenReason::WhiteText]);
        assert_eq!(out.markers, vec![31]);
    }

    #[test]
    fn not_a_docx() {
        assert!(extract_docx(b"hello").is_err());
    }
}
