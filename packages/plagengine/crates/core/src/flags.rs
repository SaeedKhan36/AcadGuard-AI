//! Integrity flags (Turnitin "Flags" panel): hidden text, replaced
//! characters and other manipulation. Only reported for documents with at
//! least [`MIN_NON_SPACE_CHARS`] non-space characters.

use crate::normalize::NormDoc;
use crate::types::{slice_chars, HiddenSpan, Span};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MIN_NON_SPACE_CHARS: usize = 500;
const MAX_LOCATIONS: usize = 1000;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FlagLocation {
    pub range: Span,
    /// Suspect characters at this location.
    pub count: usize,
    /// The affected text (for hidden text: the hidden content).
    pub text: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FlagGroup {
    /// Total suspect characters.
    pub count: usize,
    pub locations: Vec<FlagLocation>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

impl FlagGroup {
    fn push(&mut self, loc: FlagLocation) {
        self.count += loc.count;
        if self.locations.len() < MAX_LOCATIONS {
            self.locations.push(loc);
        } else {
            self.truncated = true;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Flags {
    /// False when the document is below the size threshold.
    pub eligible: bool,
    pub non_space_chars: usize,
    pub hidden_text: FlagGroup,
    pub replaced_characters: FlagGroup,
    pub other_manipulation: FlagGroup,
}

/// Whitespace-delimited word containing `pos`.
fn word_at(chars: &[char], pos: usize) -> Span {
    let mut s = pos;
    while s > 0 && !chars[s - 1].is_whitespace() {
        s -= 1;
    }
    let mut e = pos;
    while e < chars.len() && !chars[e].is_whitespace() {
        e += 1;
    }
    Span::new(s, e.max(pos + 1).min(chars.len()))
}

fn visible_word(chars: &[char], span: Span) -> String {
    chars[span.start..span.end].iter().filter(|c| crate::normalize::is_invisible(**c).is_none()).collect()
}

pub fn detect_flags(norm: &NormDoc, hidden: &[HiddenSpan]) -> Flags {
    let chars = &norm.chars;
    let non_space_chars = chars.iter().filter(|c| !c.is_whitespace()).count();
    let mut flags = Flags { eligible: non_space_chars >= MIN_NON_SPACE_CHARS, non_space_chars, ..Default::default() };
    if !flags.eligible {
        return flags;
    }

    // Hidden text: merge adjacent spans, ignore whitespace-only spans.
    let mut spans: Vec<HiddenSpan> = hidden.to_vec();
    spans.sort_by_key(|h| h.span);
    let mut merged: Vec<(Span, Vec<&'static str>)> = Vec::new();
    for h in &spans {
        let reason = match h.reason {
            crate::types::HiddenReason::Vanish => "hidden_property",
            crate::types::HiddenReason::WhiteText => "white_text",
            crate::types::HiddenReason::MatchesBackground => "text_matches_background",
            crate::types::HiddenReason::TinyFont => "tiny_font",
        };
        match merged.last_mut() {
            Some((s, rs)) if h.span.start <= s.end => {
                s.end = s.end.max(h.span.end);
                if !rs.contains(&reason) {
                    rs.push(reason);
                }
            }
            _ => merged.push((h.span, vec![reason])),
        }
    }
    for (span, reasons) in merged {
        let span = Span::new(span.start.min(chars.len()), span.end.min(chars.len()));
        let count = chars[span.start..span.end].iter().filter(|c| !c.is_whitespace()).count();
        if count > 0 {
            flags.hidden_text.push(FlagLocation { range: span, count, text: slice_chars(chars, span), detail: reasons.join(",") });
        }
    }

    // Replaced characters, grouped per word.
    let mut by_word: BTreeMap<Span, Vec<String>> = BTreeMap::new();
    for r in &norm.replaced {
        by_word
            .entry(word_at(chars, r.pos))
            .or_default()
            .push(format!("U+{:04X} '{}' → '{}'", r.original as u32, r.original, r.replacement));
    }
    for (span, details) in by_word {
        flags.replaced_characters.push(FlagLocation {
            range: span,
            count: details.len(),
            text: visible_word(chars, span),
            detail: details.join("; "),
        });
    }

    // Other manipulation: invisible chars adjacent to letters, grouped per word.
    let mut by_word: BTreeMap<Span, Vec<String>> = BTreeMap::new();
    for inv in &norm.invisible {
        let near_letter = |dir: isize| {
            let mut i = inv.pos as isize + dir;
            while i >= 0 && (i as usize) < chars.len() {
                let c = chars[i as usize];
                if crate::normalize::is_invisible(c).is_none() {
                    return c.is_alphanumeric();
                }
                i += dir;
            }
            false
        };
        if !(near_letter(-1) || near_letter(1)) {
            continue;
        }
        let kind = serde_json::to_value(inv.kind).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
        by_word.entry(word_at(chars, inv.pos)).or_default().push(format!("U+{:04X} {kind}", inv.ch as u32));
    }
    for (span, details) in by_word {
        flags.other_manipulation.push(FlagLocation {
            range: span,
            count: details.len(),
            text: visible_word(chars, span),
            detail: details.join("; "),
        });
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::normalize;
    use crate::types::HiddenReason;

    fn filler() -> String {
        "This is a perfectly ordinary sentence written by a student. ".repeat(10)
    }

    #[test]
    fn small_docs_are_not_eligible() {
        let n = normalize("tiny dосument", &[]);
        let f = detect_flags(&n, &[]);
        assert!(!f.eligible);
        assert_eq!(f.replaced_characters.count, 0);
    }

    #[test]
    fn detects_all_three_kinds() {
        let text = format!("{}The dосument has in\u{200B}visible marks andXhidden words. {}", filler(), filler());
        let x = text.chars().position(|c| c == 'X').unwrap();
        let hidden = vec![
            HiddenSpan { span: Span::new(x, x + 1), reason: HiddenReason::WhiteText },
            HiddenSpan { span: Span::new(0, 1), reason: HiddenReason::Vanish },
            HiddenSpan { span: Span::new(4, 5), reason: HiddenReason::TinyFont }, // whitespace only
        ];
        let n = normalize(&text, &hidden.iter().map(|h| h.span).collect::<Vec<_>>());
        let f = detect_flags(&n, &hidden);
        assert!(f.eligible);
        assert_eq!(f.hidden_text.count, 2);
        assert_eq!(f.hidden_text.locations.len(), 2);
        assert_eq!(f.hidden_text.locations[1].detail, "white_text");
        assert_eq!(f.replaced_characters.count, 2);
        assert_eq!(f.replaced_characters.locations.len(), 1);
        assert_eq!(f.replaced_characters.locations[0].text, "dосument");
        assert_eq!(f.other_manipulation.count, 1);
        assert_eq!(f.other_manipulation.locations[0].text, "invisible");
    }
}
